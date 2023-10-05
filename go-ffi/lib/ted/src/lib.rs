pub mod dkg;
pub mod errors;
use anyhow::Result;
use dkg::{Ack, AckOutcome, Part, PartOutcome, PubKeyMap, SyncKeyGen};
use errors::{error_to_c_string, ErrorFFIKind};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::Mutex;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::CStr,
    sync::{Arc, RwLock},
};
use threshold_crypto::{SecretKey, SignatureShare};

static mut APP_STATE: AppState = AppState { session_map: None };

struct AppState {
    session_map: Option<HashMap<u32, Session>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            session_map: Some(HashMap::new()),
        }
    }

    fn get(&self, k: u32) -> Session {
        assert_eq!(self.session_map.is_none(), false);
        let m = self.session_map.as_ref().unwrap();
        m.get(&k).cloned().unwrap()
    }

    fn insert(&mut self, k: u32, s: Session) {
        assert_eq!(self.session_map.is_none(), false);
        let m = self.session_map.as_mut().unwrap();
        m.insert(k, s);
        // self.session_map = Some(m);
    }
}

#[derive(Debug, Clone)]
struct Session {
    sk: SecretKey,
    node: Arc<Mutex<SyncKeyGen<usize>>>,
    parts: Vec<Part>,
    acks: Vec<Ack>,
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

fn init_dkg(req_body: InitDkgReq) -> Result<InitDkgResp> {
    println!("req_body {:?}", req_body);
    // Create public key with random secret
    let sk: SecretKey = rand::random();
    let p0_pk = sk.public_key();

    // Get client public key from request body, create a map of public keys
    let mut map = BTreeMap::new();
    map.insert(0, p0_pk.clone());
    map.insert(1, req_body.p1_pk);
    let pub_keys: PubKeyMap<usize, threshold_crypto::PublicKey> = Arc::new(map);

    // Create SyncKeyGen instance
    let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");
    let threshold = 0;
    let (sync_key_gen, opt_part) =
        SyncKeyGen::new(0, sk.clone(), pub_keys.clone(), threshold, &mut rng)
            .unwrap_or_else(|_| panic!("Failed to create `SyncKeyGen` instance for node #{}", 0));

    let parts = vec![opt_part.unwrap().clone()];
    let acks = vec![];

    let session = Session {
        sk,
        node: Arc::new(Mutex::new(sync_key_gen)),
        parts: parts.clone(),
        acks,
    };
    // db.write().unwrap().insert(0, session);
    // insert_m(0, session);
    unsafe {
        APP_STATE.insert(0, session);
    }

    let resp = InitDkgResp {
        p0_pk,
        p0_part: parts[0].clone(),
    };
    Ok(resp)
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn init(c_init_dkg_json: *const c_char) -> *mut c_char {
    unsafe {
        if APP_STATE.session_map.is_none() {
            APP_STATE = AppState::new();
        }
    }

    let init_dkg_json = match get_str_from_c_char(c_init_dkg_json, "init_dkg_json") {
        Ok(s) => s,
        Err(e) => return error_to_c_string(e),
    };

    let init_req: InitDkgReq = match serde_json::from_str(&init_dkg_json) {
        Ok(s) => s,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E104 {
                msg: "init_dkg".to_owned(),
                e: e.to_string(),
            })
        }
    };

    let init_dkg_resp = match init_dkg(init_req) {
        Ok(s) => s,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E103 {
                msg: "init_dkg_resp".to_owned(),
                e: e.to_string(),
            })
        }
    };

    let init_dkg_resp_json = match serde_json::to_string(&init_dkg_resp) {
        Ok(share) => share,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E103 {
                msg: "dkg_resp_json".to_owned(),
                e: e.to_string(),
            })
        }
    };

    CString::new(init_dkg_resp_json).unwrap().into_raw()
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

fn commit_dkg(req_body: CommitReq) -> Result<CommitResp> {
    println!("req_body {:?}", req_body);
    // let session = db.read().unwrap().get(&0).cloned().unwrap();
    let session = unsafe { APP_STATE.get(0) };
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
    // db.write().unwrap().insert(0, updated_session);
    unsafe {
        APP_STATE.insert(0, updated_session);
    }
    let resp = CommitResp { p0_acks: resp_acks };
    println!("resp {:?}", resp);
    Ok(resp)
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn commit(c_commit_json: *const c_char) -> *mut c_char {
    let commit_json = match get_str_from_c_char(c_commit_json, "commit_json") {
        Ok(s) => s,
        Err(e) => return error_to_c_string(e),
    };

    let commit_req: CommitReq = match serde_json::from_str(&commit_json) {
        Ok(s) => s,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E104 {
                msg: "commit_dkg".to_owned(),
                e: e.to_string(),
            })
        }
    };

    let commit_resp = match commit_dkg(commit_req) {
        Ok(s) => s,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E103 {
                msg: "commit_resp".to_owned(),
                e: e.to_string(),
            })
        }
    };

    let commit_resp_json = match serde_json::to_string(&commit_resp) {
        Ok(share) => share,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E103 {
                msg: "commit_resp_json".to_owned(),
                e: e.to_string(),
            })
        }
    };

    CString::new(commit_resp_json).unwrap().into_raw()
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
fn finalize_dkg(req_body: FinalizeReq) -> Result<FinalizeResp> {
    // let session = db.read().unwrap().get(&0).cloned().unwrap();
    let session = unsafe { APP_STATE.get(0) };
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

    Ok(FinalizeResp { is_success })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn finalize(c_finalize_json: *const c_char) -> *mut c_char {
    let finalize_json = match get_str_from_c_char(c_finalize_json, "finalize_json") {
        Ok(s) => s,
        Err(e) => return error_to_c_string(e),
    };

    let finalize_req: FinalizeReq = match serde_json::from_str(&finalize_json) {
        Ok(s) => s,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E104 {
                msg: "finalize_dkg".to_owned(),
                e: e.to_string(),
            })
        }
    };

    let finalize_resp = match finalize_dkg(finalize_req) {
        Ok(s) => s,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E103 {
                msg: "finalize_resp".to_owned(),
                e: e.to_string(),
            })
        }
    };

    let finalize_resp_json = match serde_json::to_string(&finalize_resp) {
        Ok(share) => share,
        Err(e) => {
            return error_to_c_string(ErrorFFIKind::E103 {
                msg: "finalize_resp_json".to_owned(),
                e: e.to_string(),
            })
        }
    };

    CString::new(finalize_resp_json).unwrap().into_raw()
}

pub fn get_str_from_c_char(c: *const c_char, err_msg: &str) -> Result<String, ErrorFFIKind> {
    let raw = unsafe { CStr::from_ptr(c) };
    let s = match raw.to_str() {
        Ok(s) => s,
        Err(e) => {
            return Err(ErrorFFIKind::E100 {
                msg: err_msg.to_owned(),
                e: e.to_string(),
            })
        }
    };

    Ok(s.to_string())
}

// This is present so it's easy to test that the code works natively in Rust via `cargo test`
#[cfg(test)]
pub mod test {

    use super::*;
    use std::ffi::CString;

    // This is meant to do the same stuff as the main function in the .go files
    #[test]
    fn simulated_main_function() {
        // whisper(CString::new("this is code from Rust").unwrap().into_raw());
    }
}
