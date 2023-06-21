pub mod dkg;
use axum::{
    error_handling::HandleErrorLayer, extract::State, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router,
};
use axum_macros::debug_handler;
use dkg::{Ack, NodeIdT, Part, PubKeyMap, SyncKeyGen};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use threshold_crypto::SecretKey;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type Db = Arc<RwLock<HashMap<usize, Session>>>;
struct Session {
    sk: SecretKey,
    node: SyncKeyGen<usize>,
    parts: Vec<Part>,
    acks: Vec<Ack>,
    success: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_todos=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Db::default();

    // Compose the routes
    let app = Router::new()
        .route("/init_dkg", post(init_dkg))
        .route("/commit", post(commit))
        // .route("/commit_ack", post(commit_ack))
        // .route("/finalize_dkg", post(finalize_dkg))
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
        node: sync_key_gen,
        parts,
        acks,
        success: false,
    };
    db.write().unwrap().insert(0, session);


    Json(resp)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CommitReq {
    p1_part: Part,
    p1_ack: Ack,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CommitResp {
    p0_ack: Ack,
}

async fn commit(State(db): State<Db>, Json(req_body): Json<CommitReq>) -> impl IntoResponse {
    let mut parts = db.read().unwrap().get(&0).unwrap().parts.clone();
    parts.insert(1, req_body.p1_part.clone());
    let mut acks = vec![];
    acks.insert(1, req_body.p1_ack.clone());
    // db.write().unwrap().insert(todo.id, todo.clone());
    let resp = CommitResp {
        p0_ack: acks[0].clone(),
    };
    
    Json(resp)
}
