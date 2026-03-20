//! Conflict of interest detection.

use exo_core::{Did, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDeclaration {
    pub declarant_did: Did,
    pub nature: String,
    pub related_dids: Vec<Did>,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub action_id: String,
    pub actor_did: Did,
    pub affected_dids: Vec<Did>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub declaration: ConflictDeclaration,
    pub affected_did: Did,
    pub severity: ConflictSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Advisory,
    Material,
    Disqualifying,
}

#[must_use]
pub fn check_conflicts(
    actor: &Did,
    action: &ActionRequest,
    declarations: &[ConflictDeclaration],
) -> Vec<Conflict> {
    let mut conflicts = Vec::new();
    for decl in declarations {
        if &decl.declarant_did != actor {
            continue;
        }
        for related in &decl.related_dids {
            if action.affected_dids.contains(related) {
                let severity =
                    if decl.nature.contains("financial") || decl.nature.contains("ownership") {
                        ConflictSeverity::Disqualifying
                    } else if decl.nature.contains("personal") || decl.nature.contains("family") {
                        ConflictSeverity::Material
                    } else {
                        ConflictSeverity::Advisory
                    };
                conflicts.push(Conflict {
                    declaration: decl.clone(),
                    affected_did: related.clone(),
                    severity,
                });
            }
        }
    }
    conflicts
}

#[must_use]
pub fn must_recuse(conflicts: &[Conflict]) -> bool {
    conflicts.iter().any(|c| {
        c.severity == ConflictSeverity::Disqualifying || c.severity == ConflictSeverity::Material
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn did(name: &str) -> Did {
        Did::new(&format!("did:exo:{name}")).expect("ok")
    }

    fn decl(nature: &str, related: &str) -> ConflictDeclaration {
        ConflictDeclaration {
            declarant_did: did("alice"),
            nature: nature.into(),
            related_dids: vec![did(related)],
            timestamp: Timestamp::new(1000, 0),
        }
    }
    fn action(affected: &str) -> ActionRequest {
        ActionRequest {
            action_id: "a1".into(),
            actor_did: did("alice"),
            affected_dids: vec![did(affected)],
            description: "test".into(),
        }
    }

    #[test]
    fn no_conflicts_when_none() {
        assert!(check_conflicts(&did("alice"), &action("bob"), &[]).is_empty());
    }
    #[test]
    fn financial_disqualifying() {
        let c = check_conflicts(
            &did("alice"),
            &action("bob"),
            &[decl("financial interest", "bob")],
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].severity, ConflictSeverity::Disqualifying);
        assert!(must_recuse(&c));
    }
    #[test]
    fn personal_material() {
        let c = check_conflicts(
            &did("alice"),
            &action("carol"),
            &[decl("personal relationship", "carol")],
        );
        assert_eq!(c[0].severity, ConflictSeverity::Material);
        assert!(must_recuse(&c));
    }
    #[test]
    fn advisory_no_recuse() {
        let c = check_conflicts(
            &did("alice"),
            &action("dave"),
            &[decl("acquaintance", "dave")],
        );
        assert_eq!(c[0].severity, ConflictSeverity::Advisory);
        assert!(!must_recuse(&c));
    }
    #[test]
    fn no_overlap() {
        assert!(
            check_conflicts(&did("alice"), &action("carol"), &[decl("financial", "bob")])
                .is_empty()
        );
    }
    #[test]
    fn different_actor_ignored() {
        let d = ConflictDeclaration {
            declarant_did: did("bob"),
            nature: "financial".into(),
            related_dids: vec![did("carol")],
            timestamp: Timestamp::new(1000, 0),
        };
        assert!(check_conflicts(&did("alice"), &action("carol"), &[d]).is_empty());
    }
    #[test]
    fn must_recuse_empty() {
        assert!(!must_recuse(&[]));
    }
    #[test]
    fn ownership_disqualifying() {
        let c = check_conflicts(
            &did("alice"),
            &action("bob"),
            &[decl("ownership stake", "bob")],
        );
        assert_eq!(c[0].severity, ConflictSeverity::Disqualifying);
    }
    #[test]
    fn family_material() {
        let c = check_conflicts(
            &did("alice"),
            &action("bob"),
            &[decl("family member", "bob")],
        );
        assert_eq!(c[0].severity, ConflictSeverity::Material);
    }
}
