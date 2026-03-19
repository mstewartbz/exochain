//! Constitutional document management.
use exo_core::{Did, Hash256, Signature, Timestamp};
use serde::{Deserialize, Serialize};
use crate::error::{ForumError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArticleStatus { Active, Amended, Repealed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article { pub id: String, pub title: String, pub text_hash: Hash256, pub status: ArticleStatus }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumPolicy { pub required_signatures: usize, pub required_fraction_pct: u32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constitution {
    pub version: u64,
    pub hash: Hash256,
    pub articles: Vec<Article>,
    pub ratified_at: Option<Timestamp>,
}

impl Constitution {
    #[must_use] pub fn new(articles: Vec<Article>) -> Self {
        let mut hasher = blake3::Hasher::new();
        for a in &articles { hasher.update(a.text_hash.as_bytes()); }
        Constitution { version: 1, hash: Hash256::from_bytes(*hasher.finalize().as_bytes()),
            articles, ratified_at: None }
    }
    #[must_use] pub fn is_ratified(&self) -> bool { self.ratified_at.is_some() }
}

pub fn ratify(constitution: &mut Constitution, signatures: &[(Did, Signature)], quorum: &QuorumPolicy) -> Result<()> {
    if constitution.is_ratified() { return Err(ForumError::NotRatified { reason: "already ratified".into() }); }
    let valid = signatures.iter().filter(|(_, s)| *s.as_bytes() != [0u8; 64]).count();
    if valid < quorum.required_signatures {
        return Err(ForumError::QuorumNotMet { required: quorum.required_signatures, actual: valid });
    }
    constitution.ratified_at = Some(Timestamp::ZERO);
    Ok(())
}

pub fn amend(constitution: &mut Constitution, amendment: Article, signatures: &[(Did, Signature)]) -> Result<()> {
    if !constitution.is_ratified() { return Err(ForumError::AmendmentFailed { reason: "not ratified".into() }); }
    let valid = signatures.iter().filter(|(_, s)| *s.as_bytes() != [0u8; 64]).count();
    if valid == 0 { return Err(ForumError::AmendmentFailed { reason: "no valid signatures".into() }); }
    constitution.articles.push(amendment);
    constitution.version += 1;
    // Rehash
    let mut hasher = blake3::Hasher::new();
    for a in &constitution.articles { hasher.update(a.text_hash.as_bytes()); }
    constitution.hash = Hash256::from_bytes(*hasher.finalize().as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn did(n: &str) -> Did { Did::new(&format!("did:exo:{n}")).unwrap() }
    fn sig() -> Signature { let mut s = [0u8; 64]; s[0] = 1; Signature::from_bytes(s) }
    fn article(id: &str) -> Article { Article { id: id.into(), title: id.into(), text_hash: Hash256::digest(id.as_bytes()), status: ArticleStatus::Active } }
    fn quorum() -> QuorumPolicy { QuorumPolicy { required_signatures: 2, required_fraction_pct: 50 } }

    #[test] fn new_not_ratified() { let c = Constitution::new(vec![article("a1")]); assert!(!c.is_ratified()); assert_eq!(c.version, 1); }
    #[test] fn ratify_ok() { let mut c = Constitution::new(vec![article("a1")]); ratify(&mut c, &[(did("a"), sig()), (did("b"), sig())], &quorum()).unwrap(); assert!(c.is_ratified()); }
    #[test] fn ratify_quorum_not_met() { let mut c = Constitution::new(vec![article("a1")]); assert!(ratify(&mut c, &[(did("a"), sig())], &quorum()).is_err()); }
    #[test] fn ratify_already() { let mut c = Constitution::new(vec![article("a1")]); ratify(&mut c, &[(did("a"), sig()), (did("b"), sig())], &quorum()).unwrap(); assert!(ratify(&mut c, &[(did("a"), sig()), (did("b"), sig())], &quorum()).is_err()); }
    #[test] fn amend_ok() { let mut c = Constitution::new(vec![article("a1")]); ratify(&mut c, &[(did("a"), sig()), (did("b"), sig())], &quorum()).unwrap(); let old_hash = c.hash; amend(&mut c, article("a2"), &[(did("a"), sig())]).unwrap(); assert_eq!(c.articles.len(), 2); assert_eq!(c.version, 2); assert_ne!(c.hash, old_hash); }
    #[test] fn amend_not_ratified() { let mut c = Constitution::new(vec![article("a1")]); assert!(amend(&mut c, article("a2"), &[(did("a"), sig())]).is_err()); }
    #[test] fn amend_no_sigs() { let mut c = Constitution::new(vec![article("a1")]); ratify(&mut c, &[(did("a"), sig()), (did("b"), sig())], &quorum()).unwrap(); assert!(amend(&mut c, article("a2"), &[(did("a"), Signature::from_bytes([0u8; 64]))]).is_err()); }
    #[test] fn article_status_serde() { for s in [ArticleStatus::Active, ArticleStatus::Amended, ArticleStatus::Repealed] { let j = serde_json::to_string(&s).unwrap(); let r: ArticleStatus = serde_json::from_str(&j).unwrap(); assert_eq!(r, s); } }
    #[test] fn constitution_hash_deterministic() { let c1 = Constitution::new(vec![article("a1")]); let c2 = Constitution::new(vec![article("a1")]); assert_eq!(c1.hash, c2.hash); }
    #[test] fn empty_sig_not_counted() { let mut c = Constitution::new(vec![article("a1")]); let empty = Signature::from_bytes([0u8; 64]); assert!(ratify(&mut c, &[(did("a"), sig()), (did("b"), empty)], &quorum()).is_err()); }
}
