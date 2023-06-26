pub mod dkg;
pub mod ted;
use axum::{
    error_handling::HandleErrorLayer, extract::State, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router,
};
use axum_macros::debug_handler;
use dkg::{Ack, AckOutcome, Part, PartOutcome, PubKeyMap, SyncKeyGen};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use threshold_crypto::{SecretKey, PublicKeyShare};
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

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct InitDkgReq {
    p1_pk: threshold_crypto::PublicKey,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct InitDkgResp {
    p0_pk: threshold_crypto::PublicKey,
    p0_part: Part,
}
#[debug_handler]
async fn init_dkg(State(db): State<Db>, Json(req_body): Json<InitDkgReq>) -> impl IntoResponse {
    println!("req_body {:?}", req_body);
    let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");
    let threshold = 0;
    let sk: SecretKey = rand::random();
    let p0_pk = sk.public_key();
    let mut map = BTreeMap::new();
    map.insert(0, p0_pk.clone());
    map.insert(1, req_body.p1_pk.clone());
    let pub_keys: PubKeyMap<usize, threshold_crypto::PublicKey> = Arc::new(map);
    let (sync_key_gen, opt_part) =
        SyncKeyGen::new(0, sk.clone(), pub_keys.clone(), threshold, &mut rng)
            .unwrap_or_else(|_| panic!("Failed to create `SyncKeyGen` instance for node #{}", 0));
    let parts = vec![opt_part.unwrap().clone()];
    let resp = InitDkgResp {
        p0_pk: p0_pk.clone(),
        p0_part: parts[0].clone(),
    };
    let acks = vec![];
    let session = Session {
        sk,
        node: Arc::new(Mutex::new(sync_key_gen)),
        parts,
        acks,
    };
    db.write().unwrap().insert(0, session);

    Json(resp)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CommitReq {
    p1_part: Part,
    p1_acks: Vec<Ack>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CommitResp {
    p0_acks: Vec<Ack>,
}

async fn commit(State(db): State<Db>, Json(req_body): Json<CommitReq>) -> impl IntoResponse {
    println!("req_body {:?}", req_body);
    let session = db.read().unwrap().get(&0).cloned().unwrap();
    let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");

    let mut parts = session.parts;
    parts.insert(1, req_body.p1_part.clone());

    let arc_node = session.node.clone();
    let mut node = arc_node.try_lock().unwrap();

    let mut acks = vec![];

    for (id, part) in parts.clone().iter().enumerate() {
        // We only have 2 participants
            match node
                .handle_part(&id, part.clone(), &mut rng)
                .expect("Failed to handle Part")
            {
                PartOutcome::Valid(Some(ack)) => acks.push(ack),
                PartOutcome::Invalid(fault) => panic!(
                    "Node #0 handles Part from node #{} and detects a fault: {:?}",
                    id, fault
                ),
                PartOutcome::Valid(None) => {
                    panic!("We are not an observer, so we should send Ack.")
                }
            }
    }

    for ack in req_body.p1_acks.into_iter() {
        acks.push(ack);
    }
    let resp_acks = acks.clone();

    let updated_session = Session {
        sk: session.sk,
        node: session.node,
        parts,
        acks,
    };
    db.write().unwrap().insert(0, updated_session);
    let resp = CommitResp { p0_acks: resp_acks };

    Json(resp)
}


#[derive(Debug, Deserialize, Serialize, Clone)]
struct FinalizeReq {
    pks_1: PublicKeyShare
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FinalizeResp {
    is_success: bool
}
async fn finalize_dkg(State(db): State<Db>, Json(req_body): Json<FinalizeReq>) -> impl IntoResponse {
    println!("req_body {:?}", req_body);
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
        .expect("Failed to create `PublicKeySet` from node #0")
        .0;
    assert!(node.is_ready());
    let (pks, opt_sks) = node.generate().unwrap_or_else(|_| {
        panic!("Failed to create `PublicKeySet` and `SecretKeyShare` for node #0")
    });
    assert_eq!(pks, pub_key_set); // All nodes now know the public keys and public key shares.
    let msg = "Sign this";
    let sks_0 = opt_sks.expect("Not an observer node: We receive a secret key share.");
    let sig_share = sks_0.sign(msg);
    let pks_0 = pub_key_set.public_key_share(0);
    let pks_1 = req_body.pks_1;
    let is_success_pks_0 = pks_0.verify(&sig_share, msg);
    let is_success_pks_1 = pks_1.verify(&sig_share, msg);
    let is_success = is_success_pks_0 && is_success_pks_1;
    println!("is_success {:?}", is_success);
    Json(FinalizeResp { is_success })
}
