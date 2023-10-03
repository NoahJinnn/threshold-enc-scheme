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
type Db = Arc<RwLock<HashMap<usize, Session>>>;

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

    let session = Session {
        sk,
        node: Arc::new(Mutex::new(sync_key_gen)),
        parts: parts.clone(),
        acks,
    };
    // db.write().unwrap().insert(0, session);

    let resp = InitDkgResp {
        p0_pk: p0_pk.clone(),
        p0_part: parts[0].clone(),
    };
    Ok(resp)
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn initdkg(c_init_dkg_json: *const c_char) -> *mut c_char {

    let init_dkg_json = match get_str_from_c_char(c_init_dkg_json, "master_key_json") {
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
                msg: "signature_json".to_owned(),
                e: e.to_string(),
            })
        }
    };

    println!("init_dkg_resp_json {:?}", init_dkg_resp_json);

    CString::new(init_dkg_resp_json).unwrap().into_raw()
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
// #[no_mangle]
// pub extern "C" fn hello(name: *const c_char) {
//     let name_cstr = unsafe { CStr::from_ptr(name) };
//     let name = name_cstr.to_str().unwrap();
//     println!("Hello {}!", name);
// }

// #[no_mangle]
// pub extern "C" fn whisper(message: *const c_char) {
//     let message_cstr = unsafe { CStr::from_ptr(message) };
//     let message = message_cstr.to_str().unwrap();
//     println!("({})", message);
// }



// This is present so it's easy to test that the code works natively in Rust via `cargo test`
#[cfg(test)]
pub mod test {

    use super::*;
    use std::ffi::CString;

    // This is meant to do the same stuff as the main function in the .go files
    #[test]
    fn simulated_main_function() {
        // hello(CString::new("world").unwrap().into_raw());
        // whisper(CString::new("this is code from Rust").unwrap().into_raw());
    }
}
