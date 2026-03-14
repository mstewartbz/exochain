#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthorityLink {
    pub pubkey: String,
    pub signature: String,
}
