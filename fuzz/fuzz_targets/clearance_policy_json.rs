#![no_main]

use exo_core::Did;
use exo_governance::clearance::{ClearancePolicy, ClearanceRegistry, check_clearance};
use libfuzzer_sys::fuzz_target;

const MAX_FUZZ_JSON_BYTES: usize = 16 * 1024;
const MAX_ACTIONS_TO_EVALUATE: usize = 32;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_FUZZ_JSON_BYTES {
        return;
    }

    let policy: Result<ClearancePolicy, _> = serde_json::from_slice(data);
    let Ok(policy) = policy else {
        return;
    };
    let Ok(actor) = Did::new("did:exo:fuzz-clearance-actor") else {
        return;
    };

    let registry = ClearanceRegistry::default();
    for action in policy.actions.keys().take(MAX_ACTIONS_TO_EVALUATE) {
        let _ = check_clearance(&actor, action, &policy, &registry);
    }
});
