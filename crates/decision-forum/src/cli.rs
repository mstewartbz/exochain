use crate::create_genesis_decision;

pub fn run() {
    println!("🚀 decision.forum engine starting...");
    match create_genesis_decision("First Board Resolution — Governance Substrate Birth") {
        Ok(obj) => {
            println!("✅ Genesis Decision Object born: {}", obj.id);
            println!("Fiduciary Defense Package:\n{}",
                crate::fiduciary_package::FiduciaryDefensePackage::generate(&obj));
        }
        Err(e) => eprintln!("TNC violation: {}", e),
    }
}
