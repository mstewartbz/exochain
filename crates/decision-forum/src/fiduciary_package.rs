use super::DecisionObject;

pub struct FiduciaryDefensePackage;

impl FiduciaryDefensePackage {
    pub fn generate(obj: &DecisionObject) -> String {
        format!("Fiduciary Defense Package for {} (FRE 803(6) compliant)\nAuthority: {}\nEvidence hash: {}",
            obj.title, obj.authority_chain.len(), obj.merkle_root)
    }
}
