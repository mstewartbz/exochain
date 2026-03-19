//! Peer-to-peer mesh networking.
use exo_core::{Did, Hash256, Signature, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::error::{ApiError, Result};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PeerId(pub Did);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub addresses: Vec<String>,
    pub public_key_hash: Hash256,
    pub last_seen: Timestamp,
    pub reputation_score: u32,
}

#[derive(Debug, Clone, Default)]
pub struct PeerRegistry { pub peers: BTreeMap<PeerId, PeerInfo> }

impl PeerRegistry {
    #[must_use] pub fn new() -> Self { Self { peers: BTreeMap::new() } }
    pub fn register(&mut self, info: PeerInfo) { self.peers.insert(info.id.clone(), info); }
    #[must_use] pub fn get(&self, id: &PeerId) -> Option<&PeerInfo> { self.peers.get(id) }
    #[must_use] pub fn len(&self) -> usize { self.peers.len() }
    #[must_use] pub fn is_empty(&self) -> bool { self.peers.is_empty() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub from: PeerId,
    pub to: Option<PeerId>,
    pub payload: Vec<u8>,
    pub signature: Signature,
    pub nonce: u64,
}

/// Rate-limit tracking per peer.
#[derive(Debug, Default)]
pub struct RateLimiter { counts: BTreeMap<PeerId, u32> }
impl RateLimiter {
    const MAX_PER_WINDOW: u32 = 100;
    #[must_use] pub fn new() -> Self { Self { counts: BTreeMap::new() } }
    pub fn check_and_increment(&mut self, peer: &PeerId) -> Result<()> {
        let c = self.counts.entry(peer.clone()).or_insert(0);
        if *c >= Self::MAX_PER_WINDOW {
            return Err(ApiError::RateLimited { peer_id: format!("{:?}", peer) });
        }
        *c += 1;
        Ok(())
    }
    pub fn reset(&mut self) { self.counts.clear(); }
}

pub fn send(registry: &PeerRegistry, msg: &Message) -> Result<()> {
    if let Some(ref to) = msg.to {
        if !registry.peers.contains_key(to) {
            return Err(ApiError::PeerNotFound(format!("{to:?}")));
        }
    }
    Ok(())
}

pub fn verify_message(msg: &Message) -> Result<()> {
    // Verify signature is non-zero (placeholder for real crypto verification)
    if *msg.signature.as_bytes() == [0u8; 64] {
        return Err(ApiError::VerificationFailed { reason: "empty signature".into() });
    }
    Ok(())
}

pub fn discover_peers(registry: &mut PeerRegistry, bootstrap: &[String]) -> Result<Vec<PeerId>> {
    let mut discovered = Vec::new();
    for addr in bootstrap {
        let did = Did::new(&format!("did:exo:peer-{}", blake3::hash(addr.as_bytes()).to_hex())).unwrap();
        let pid = PeerId(did);
        if !registry.peers.contains_key(&pid) {
            registry.register(PeerInfo {
                id: pid.clone(), addresses: vec![addr.clone()],
                public_key_hash: Hash256::digest(addr.as_bytes()),
                last_seen: Timestamp::ZERO, reputation_score: 50,
            });
            discovered.push(pid);
        }
    }
    Ok(discovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pid(n: &str) -> PeerId { PeerId(Did::new(&format!("did:exo:{n}")).unwrap()) }
    fn info(n: &str) -> PeerInfo { PeerInfo { id: pid(n), addresses: vec!["addr".into()], public_key_hash: Hash256::ZERO, last_seen: Timestamp::ZERO, reputation_score: 50 } }
    fn msg(from: &str, to: Option<&str>) -> Message {
        let mut sig = [0u8; 64]; sig[0] = 1;
        Message { from: pid(from), to: to.map(pid), payload: b"hello".to_vec(), signature: Signature::from_bytes(sig), nonce: 1 }
    }

    #[test] fn registry_empty() { let r = PeerRegistry::new(); assert!(r.is_empty()); assert_eq!(r.len(), 0); }
    #[test] fn registry_register() { let mut r = PeerRegistry::new(); r.register(info("a")); assert_eq!(r.len(), 1); assert!(r.get(&pid("a")).is_some()); }
    #[test] fn registry_default() { assert!(PeerRegistry::default().is_empty()); }
    #[test] fn send_known_peer() { let mut r = PeerRegistry::new(); r.register(info("b")); assert!(send(&r, &msg("a", Some("b"))).is_ok()); }
    #[test] fn send_unknown_peer() { let r = PeerRegistry::new(); assert!(send(&r, &msg("a", Some("b"))).is_err()); }
    #[test] fn send_broadcast() { let r = PeerRegistry::new(); assert!(send(&r, &msg("a", None)).is_ok()); }
    #[test] fn verify_ok() { assert!(verify_message(&msg("a", None)).is_ok()); }
    #[test] fn verify_empty_sig() {
        let m = Message { from: pid("a"), to: None, payload: vec![], signature: Signature::from_bytes([0u8; 64]), nonce: 0 };
        assert!(verify_message(&m).is_err());
    }
    #[test] fn discover() { let mut r = PeerRegistry::new(); let d = discover_peers(&mut r, &["addr1".into(), "addr2".into()]).unwrap(); assert_eq!(d.len(), 2); assert_eq!(r.len(), 2); }
    #[test] fn discover_no_dupes() { let mut r = PeerRegistry::new(); discover_peers(&mut r, &["a".into()]).unwrap(); let d = discover_peers(&mut r, &["a".into()]).unwrap(); assert!(d.is_empty()); }
    #[test] fn rate_limiter() { let mut rl = RateLimiter::new(); for _ in 0..100 { rl.check_and_increment(&pid("a")).unwrap(); } assert!(rl.check_and_increment(&pid("a")).is_err()); }
    #[test] fn rate_limiter_reset() { let mut rl = RateLimiter::new(); for _ in 0..100 { rl.check_and_increment(&pid("a")).unwrap(); } rl.reset(); assert!(rl.check_and_increment(&pid("a")).is_ok()); }
    #[test] fn peer_id_ord() { assert!(pid("a") < pid("b")); }
    #[test] fn message_serde() { let m = msg("a", Some("b")); let j = serde_json::to_string(&m).unwrap(); assert!(!j.is_empty()); }
}
