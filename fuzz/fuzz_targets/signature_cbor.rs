#![no_main]

use exo_core::Signature;
use libfuzzer_sys::fuzz_target;

const MAX_FUZZ_CBOR_BYTES: usize = 8 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_FUZZ_CBOR_BYTES {
        return;
    }

    let decoded: Result<Signature, _> = ciborium::from_reader(data);
    if let Ok(signature) = decoded {
        let _ = signature.algorithm();
        let _ = signature.ed25519_bytes();
        let _ = signature.ed25519_component_is_zero();
        let _ = signature.is_empty();
        let _ = signature.to_bytes();
    }
});
