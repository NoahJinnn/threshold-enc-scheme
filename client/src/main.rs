pub mod dkg;
use axum::{
    error_handling::HandleErrorLayer, extract::State, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router,
};
use axum_macros::debug_handler;
use dkg::{Ack, AckOutcome, Part, PartOutcome, PubKeyMap, SyncKeyGen};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use threshold_crypto::{SecretKey, SignatureShare};
use tokio::sync::Mutex;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;

type Db = Arc<RwLock<HashMap<usize, Session>>>;

#[derive(Debug, Clone)]
struct Session {
    sk: SecretKey,
    node: Arc<Mutex<SyncKeyGen<usize>>>,
    parts: Vec<Part>,
    acks: Vec<Ack>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let db = Db::default();

    // Compose the routes
    let app = Router::new()
        .route("/init_dkg", post(init_dkg))
        .route("/commit", post(commit))
        .route("/finalize_dkg", post(finalize_dkg))
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        .with_state(db);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Deserialize, Serialize)]
struct InitDkgReq {
    p1_pk: threshold_crypto::PublicKey,
}
#[debug_handler]
async fn init_dkg(State(db): State<Db>) -> impl IntoResponse {
    let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");
    let threshold = 0;
    let sk: SecretKey = rand::random();
    let p1_pk = sk.public_key();
    let mut map = BTreeMap::new();

    // // Send req to server
    let req_body = InitDkgReq {
        p1_pk: p1_pk.clone(),
    };
    let dkg_init_resp: InitDkgResp = init_dkg_req("http://127.0.0.1:3000", &req_body)
        .await
        .unwrap();

    map.insert(0, dkg_init_resp.p0_pk);
    map.insert(1, p1_pk.clone());
    let pub_keys: PubKeyMap<usize, threshold_crypto::PublicKey> = Arc::new(map);
    let (sync_key_gen, opt_part) =
        SyncKeyGen::new(1, sk.clone(), pub_keys.clone(), threshold, &mut rng)
            .unwrap_or_else(|_| panic!("Failed to create `SyncKeyGen` instance for node #{}", 1));
    let p1_part = opt_part.unwrap().clone();
    let parts = vec![dkg_init_resp.p0_part, p1_part];

    let acks = vec![];
    let session = Session {
        sk,
        node: Arc::new(Mutex::new(sync_key_gen)),
        parts,
        acks,
    };
    db.write().unwrap().insert(0, session);

    Json(())
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CommitReq {
    p1_part: Part,
    p1_acks: Vec<Ack>,
}
#[debug_handler]
async fn commit(State(db): State<Db>) -> impl IntoResponse {
    let session = db.read().unwrap().get(&0).cloned().unwrap();
    let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");

    let parts = session.parts;
    let arc_node = session.node.clone();
    let mut node = arc_node.try_lock().unwrap();
    let mut p1_acks = vec![];
    for (id, part) in parts.clone().iter().enumerate() {
        // We only have 2 participants
        match node
            .handle_part(&id, part.clone(), &mut rng)
            .expect("Failed to handle Part")
        {
            PartOutcome::Valid(Some(ack)) => {
                println!("Node #1 handles Part from node success #{}", id);
                p1_acks.push(ack)
            }
            PartOutcome::Invalid(fault) => {
                panic!(
                    "Node #1 handles Part from node #{} and detects a fault: {:?}",
                    id, fault
                )
            }
            PartOutcome::Valid(None) => {
                panic!("We are not an observer, so we should send Ack.")
            }
        }
    }

    // Send req to server
    let req_body = CommitReq {
        p1_part: parts[1].clone(),
        p1_acks: p1_acks.clone(),
    };
    let commit_resp: CommitResp = commit_req("http://127.0.0.1:3000", &req_body)
        .await
        .unwrap();
    let p0_acks = commit_resp.p0_acks;

    let mut acks = vec![];
    for ack in p0_acks {
        acks.push(ack);
    }

    for ack in p1_acks {
        acks.push(ack);
    }

    let updated_session = Session {
        sk: session.sk,
        node: session.node,
        parts,
        acks,
    };
    db.write().unwrap().insert(0, updated_session);
    Json(())
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FinalizeReq {
    sig_share_1: SignatureShare,
    signed_msg_1: String,
}

async fn finalize_dkg(State(db): State<Db>) -> impl IntoResponse {
    let session = db.read().unwrap().get(&0).cloned().unwrap();
    let arc_node = session.node.clone();
    let mut node = arc_node.try_lock().unwrap();
    let acks = session.acks;
    // Finally, we handle all the `Ack`s.
    for ack in acks {
        for id in 0..1 {
            match node
                .handle_ack(&id, ack.clone())
                .expect("Failed to handle Ack")
            {
                AckOutcome::Valid => (),
                AckOutcome::Invalid(fault) => panic!("Invalid Ack: {:?}", fault),
            }
        }
    }
    let pub_key_set = node
        .generate()
        .expect("Failed to create `PublicKeySet` from node #1")
        .0;
    assert!(node.is_ready());
    let (pks, opt_sks) = node.generate().unwrap_or_else(|_| {
        panic!("Failed to create `PublicKeySet` and `SecretKeyShare` for node #1")
    });
    assert_eq!(pks, pub_key_set); // All nodes now know the public keys and public key shares.

    let sks_1 = opt_sks.expect("Not an observer node: We receive a secret key share.");
    let msg = "Sign this";
    let sig_share_1 = sks_1.sign(msg);

    // Send req to server
    let req_body = FinalizeReq {
        sig_share_1,
        signed_msg_1: msg.to_string(),
    };
    let is_success = finalize_dkg_req("http://127.0.0.1:3000", &req_body)
        .await
        .unwrap();
    println!("is_success: {:?}", is_success);
    Json(())
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct InitDkgResp {
    p0_pk: threshold_crypto::PublicKey,
    p0_part: Part,
}

async fn init_dkg_req(domain: &str, body: &InitDkgReq) -> Result<InitDkgResp, Box<dyn Error>> {
    let url = format!("{}/init_dkg", domain);
    let client: Client = Client::new();
    let response = client.post(&url).json(body).send().await?;
    let response_text = response.text().await?;
    let resp: InitDkgResp = serde_json::from_str(&response_text)?;
    Ok(resp)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CommitResp {
    p0_acks: Vec<Ack>,
}
async fn commit_req(domain: &str, body: &CommitReq) -> Result<CommitResp, Box<dyn Error>> {
    let url = format!("{}/commit", domain);
    let client = Client::new();
    let response = client.post(&url).json(body).send().await?;
    let response_text = response.text().await?;
    let resp: CommitResp = serde_json::from_str(&response_text)?;
    Ok(resp)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FinalizeResp {
    is_success: bool,
}
async fn finalize_dkg_req(
    domain: &str,
    body: &FinalizeReq,
) -> Result<FinalizeResp, Box<dyn Error>> {
    let url = format!("{}/finalize_dkg", domain);
    let client = Client::new();
    let response = client.post(&url).json(body).send().await?;
    let response_text = response.text().await?;
    let resp: FinalizeResp = serde_json::from_str(&response_text)?;
    Ok(resp)
}
