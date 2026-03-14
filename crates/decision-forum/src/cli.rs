use crate::create_genesis_decision;

pub fn run() {
    #[cfg(test)]
    crate::requirements::Requirement::CliRun.mark_covered();

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

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::requirements::Requirement;

    #[test]
    pub fn test_cli_run() {
        run();
        Requirement::CliRun.mark_covered();
    }
}
