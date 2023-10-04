pub mod dkg;
pub mod sqlite;
use axum::{
    error_handling::HandleErrorLayer, extract::State, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router,
};
use axum_macros::debug_handler;
use dkg::{Ack, AckOutcome, Part, PartOutcome, PubKeyMap, SyncKeyGen};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use threshold_crypto::{SecretKey, SignatureShare};
use tokio::sync::Mutex;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<u32, Session>> = {
        let m = Mutex::new(HashMap::new());
        m
    };
}

fn insert_m(k: u32, s: Session) {
    println!("s {:?}", s);
    let mut m = HASHMAP.try_lock().unwrap().clone();
    m.insert(k, s);
    assert_eq!(m.is_empty(), false);

    println!("hashmap {:?}", m);
}

fn get_m(k: u32) -> Session {
    let m = HASHMAP.try_lock().unwrap().clone();
    println!("hashmap {:?}", m);
    m.get(&k).cloned().unwrap()
}

#[derive(Debug, Clone)]
struct Session {
    sk: SecretKey,
    node: Arc<Mutex<SyncKeyGen<usize>>>,
    parts: Vec<Part>,
    acks: Vec<Ack>,
}

type Db = Arc<RwLock<HashMap<usize, Session>>>;

// #[derive(Debug, Clone)]
// struct Session {
//     sk: SecretKey,
//     node: Arc<Mutex<SyncKeyGen<usize>>>,
//     parts: Vec<Part>,
//     acks: Vec<Ack>,
// }

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
    let req_body_json = match serde_json::to_string(&req_body) {
        Ok(share) => share,
        Err(e) => {
            println!("Error: {}", e);
            "".to_string()
        }
    };
    println!("req_body {:?}", req_body_json);

    // Create public key with random secret
    let sk: SecretKey = rand::random();
    let p0_pk = sk.public_key();

    // Get client public key from request body, create a map of public keys
    let mut map = BTreeMap::new();
    map.insert(0, p0_pk.clone());
    map.insert(1, req_body.p1_pk.clone());
    let pub_keys: PubKeyMap<usize, threshold_crypto::PublicKey> = Arc::new(map);

    // Create SyncKeyGen instance
    let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");
    let threshold = 0;
    let (sync_key_gen, opt_part) =
        SyncKeyGen::new(0, sk.clone(), pub_keys.clone(), threshold, &mut rng)
            .unwrap_or_else(|_| panic!("Failed to create `SyncKeyGen` instance for node #{}", 0));

    let parts = vec![opt_part.unwrap().clone()];
    let acks = vec![];
    // let session = Session {
    //     sk,
    //     node: Arc::new(Mutex::new(sync_key_gen)),
    //     parts: parts.clone(),
    //     acks,
    // };

    let session = Session {
        sk,
        node: Arc::new(Mutex::new(sync_key_gen)),
        parts: parts.clone(),
        acks,
    };

    // db.write().unwrap().insert(0, session);
    insert_m(0, session);

    let resp = InitDkgResp {
        p0_pk: p0_pk.clone(),
        p0_part: parts[0].clone(),
    };
    println!("resp {:?}", resp);
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
    // let session = db.read().unwrap().get(&0).cloned().unwrap();
    let session = get_m(0);
    let arc_node = session.node.clone();
    // let arc_node = session.node.clone();
    let mut node = arc_node.try_lock().unwrap();

    let mut parts = session.parts.clone();
    parts.insert(1, req_body.p1_part.clone());

    let mut acks = vec![];
    let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");

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
        sk: session.sk.clone(),
        node: session.node.clone(),
        parts,
        acks,
    };

    // db.write().unwrap().insert(0, updated_session);
    insert_m(0, updated_session);

    let resp = CommitResp { p0_acks: resp_acks };
    println!("resp {:?}", resp);
    Json(resp)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FinalizeReq {
    sig_share_1: SignatureShare,
    signed_msg_1: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FinalizeResp {
    is_success: bool,
}
async fn finalize_dkg(
    State(db): State<Db>,
    Json(req_body): Json<FinalizeReq>,
) -> impl IntoResponse {
    println!("req_body {:?}", req_body);

    // let session = db.read().unwrap().get(&0).cloned().unwrap();
    let session = get_m(0);

    let arc_node = session.node.clone();
    let mut node = arc_node.try_lock().unwrap();
    let acks = session.acks.clone();

    // we handle all the `Ack`s.
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

    let sks_0 = opt_sks.expect("Not an observer node: We receive a secret key share.");
    let sig_share_0 = sks_0.sign(req_body.signed_msg_1.clone());
    let mut sig_shares: BTreeMap<usize, SignatureShare> = BTreeMap::new();
    sig_shares.insert(0, sig_share_0);
    sig_shares.insert(1, req_body.sig_share_1);
    let combine_sig = pub_key_set
        .combine_signatures(&sig_shares)
        .expect("The shares can be combined.");

    let is_success = pub_key_set
        .public_key()
        .verify(&combine_sig, req_body.signed_msg_1);
    println!("is_success {:?}", is_success);

    Json(FinalizeResp { is_success })
}
