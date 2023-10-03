pub mod dkg;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::os::raw::c_char;
use threshold_crypto::{SecretKey, SignatureShare};
use dkg::{Ack, AckOutcome, Part, PartOutcome, PubKeyMap, SyncKeyGen};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct InitDkgReq {
    p1_pk: threshold_crypto::PublicKey,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct InitDkgResp {
    p0_pk: threshold_crypto::PublicKey,
    p0_part: Part,
}

#[no_mangle]
pub extern "C" fn hello(name: *const c_char) {
    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name = name_cstr.to_str().unwrap();
    println!("Hello {}!", name);
}

#[no_mangle]
pub extern "C" fn whisper(message: *const c_char) {
    let message_cstr = unsafe { CStr::from_ptr(message) };
    let message = message_cstr.to_str().unwrap();
    println!("({})", message);
}

// This is present so it's easy to test that the code works natively in Rust via `cargo test`
#[cfg(test)]
pub mod test {

    use super::*;
    use std::ffi::CString;

    // This is meant to do the same stuff as the main function in the .go files
    #[test]
    fn simulated_main_function() {
        hello(CString::new("world").unwrap().into_raw());
        whisper(CString::new("this is code from Rust").unwrap().into_raw());
    }
}
