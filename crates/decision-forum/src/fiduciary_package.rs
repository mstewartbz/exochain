use super::DecisionObject;

pub struct FiduciaryDefensePackage;

impl FiduciaryDefensePackage {
    pub fn generate(obj: &DecisionObject) -> String {
        #[cfg(test)]
        crate::requirements::Requirement::FiduciaryPackageGeneration.mark_covered();

        format!(
            "Fiduciary Defense Summary for {}\nAuthority count: {}\nMerkle root: {}\nNote: FRE 803(6) review required before any evidentiary compliance claim.",
            obj.title,
            obj.authority_chain.len(),
            obj.merkle_root
        )
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::requirements::Requirement;

    #[test]
    pub fn test_fiduciary_package() {
        let obj = DecisionObject::new("Fiduciary Title");
        let pkg = FiduciaryDefensePackage::generate(&obj);
        assert!(pkg.contains("Fiduciary Title"));
        assert!(pkg.contains("Authority count: 0"));
        assert!(pkg.contains(&obj.merkle_root));
        assert!(pkg.contains("FRE 803(6) review required"));
        Requirement::FiduciaryPackageGeneration.mark_covered();
    }
}
