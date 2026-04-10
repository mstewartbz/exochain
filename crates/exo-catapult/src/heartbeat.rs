//! Heartbeat monitoring — Paperclip concept adapted for ExoChain.
//!
//! Tracks per-agent pulse via HLC timestamps. Timeouts trigger PACE
//! escalation through the ODA command hierarchy.

use std::collections::BTreeMap;

use exo_core::{Did, Hash256, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a heartbeat invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HeartbeatStatus {
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
}

/// A single heartbeat record from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRecord {
    pub id: Uuid,
    pub newco_id: Uuid,
    pub agent_did: Did,
    pub status: HeartbeatStatus,
    pub started: Timestamp,
    pub finished: Option<Timestamp>,
    /// Resource usage counters (token counts, API calls, etc.).
    pub usage: BTreeMap<String, u64>,
    /// Hash of this heartbeat for receipt chaining.
    pub receipt_hash: Hash256,
}

/// An alert generated when an agent misses their heartbeat window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatAlert {
    pub agent_did: Did,
    pub last_seen: Timestamp,
    pub elapsed_ms: u64,
    pub severity: AlertSeverity,
}

/// Severity of a heartbeat alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlertSeverity {
    /// Agent is late but within tolerance.
    Warning,
    /// Agent has exceeded the timeout — PACE escalation needed.
    Critical,
}

/// Monitors agent heartbeats and generates alerts on timeout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMonitor {
    /// Last heartbeat timestamp per agent DID.
    last_seen: BTreeMap<Did, Timestamp>,
    /// Full heartbeat history per agent.
    history: BTreeMap<Did, Vec<HeartbeatRecord>>,
    /// Warning threshold in milliseconds.
    warn_ms: u64,
    /// Critical timeout in milliseconds (triggers PACE escalation).
    timeout_ms: u64,
}

impl HeartbeatMonitor {
    /// Default warning threshold: 3 minutes.
    pub const DEFAULT_WARN_MS: u64 = 180_000;
    /// Default critical timeout: 5 minutes.
    pub const DEFAULT_TIMEOUT_MS: u64 = 300_000;

    /// Create a new heartbeat monitor with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_seen: BTreeMap::new(),
            history: BTreeMap::new(),
            warn_ms: Self::DEFAULT_WARN_MS,
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }

    /// Create a monitor with custom thresholds.
    #[must_use]
    pub fn with_thresholds(warn_ms: u64, timeout_ms: u64) -> Self {
        Self {
            last_seen: BTreeMap::new(),
            history: BTreeMap::new(),
            warn_ms,
            timeout_ms,
        }
    }

    /// Record a heartbeat from an agent.
    pub fn record(&mut self, record: HeartbeatRecord) {
        let did = record.agent_did.clone();
        let ts = record.started;
        self.last_seen.insert(did.clone(), ts);
        self.history.entry(did).or_default().push(record);
    }

    /// Check all agents for heartbeat health at the given time.
    #[must_use]
    pub fn check_health(&self, now: &Timestamp) -> Vec<HeartbeatAlert> {
        let mut alerts = Vec::new();
        for (did, last) in &self.last_seen {
            let elapsed_ms = now.physical_ms.saturating_sub(last.physical_ms);
            if elapsed_ms >= self.timeout_ms {
                alerts.push(HeartbeatAlert {
                    agent_did: did.clone(),
                    last_seen: *last,
                    elapsed_ms,
                    severity: AlertSeverity::Critical,
                });
            } else if elapsed_ms >= self.warn_ms {
                alerts.push(HeartbeatAlert {
                    agent_did: did.clone(),
                    last_seen: *last,
                    elapsed_ms,
                    severity: AlertSeverity::Warning,
                });
            }
        }
        alerts
    }

    /// Get the last-seen timestamp for an agent.
    #[must_use]
    pub fn last_seen(&self, did: &Did) -> Option<&Timestamp> {
        self.last_seen.get(did)
    }

    /// Get the heartbeat history for an agent.
    #[must_use]
    pub fn history(&self, did: &Did) -> Option<&[HeartbeatRecord]> {
        self.history.get(did).map(|v| v.as_slice())
    }

    /// Number of agents being monitored.
    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.last_seen.len()
    }
}

impl Default for HeartbeatMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did(name: &str) -> Did {
        Did::new(&format!("did:exo:test-{name}")).unwrap()
    }

    fn make_heartbeat(did: Did, time_ms: u64) -> HeartbeatRecord {
        HeartbeatRecord {
            id: Uuid::new_v4(),
            newco_id: Uuid::nil(),
            agent_did: did,
            status: HeartbeatStatus::Completed,
            started: Timestamp {
                physical_ms: time_ms,
                logical: 0,
            },
            finished: Some(Timestamp {
                physical_ms: time_ms + 100,
                logical: 0,
            }),
            usage: BTreeMap::new(),
            receipt_hash: Hash256::ZERO,
        }
    }

    #[test]
    fn record_and_last_seen() {
        let mut monitor = HeartbeatMonitor::new();
        let did = test_did("agent1");
        monitor.record(make_heartbeat(did.clone(), 1000));
        assert_eq!(monitor.last_seen(&did).unwrap().physical_ms, 1000);
        assert_eq!(monitor.agent_count(), 1);
    }

    #[test]
    fn no_alerts_when_healthy() {
        let mut monitor = HeartbeatMonitor::new();
        let did = test_did("agent1");
        monitor.record(make_heartbeat(did, 1000));

        let now = Timestamp {
            physical_ms: 1500,
            logical: 0,
        };
        assert!(monitor.check_health(&now).is_empty());
    }

    #[test]
    fn warning_alert() {
        let mut monitor = HeartbeatMonitor::with_thresholds(100, 200);
        let did = test_did("agent1");
        monitor.record(make_heartbeat(did, 1000));

        let now = Timestamp {
            physical_ms: 1150,
            logical: 0,
        };
        let alerts = monitor.check_health(&now);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
    }

    #[test]
    fn critical_alert() {
        let mut monitor = HeartbeatMonitor::with_thresholds(100, 200);
        let did = test_did("agent1");
        monitor.record(make_heartbeat(did, 1000));

        let now = Timestamp {
            physical_ms: 1250,
            logical: 0,
        };
        let alerts = monitor.check_health(&now);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }

    #[test]
    fn multiple_agents() {
        let mut monitor = HeartbeatMonitor::with_thresholds(100, 200);
        monitor.record(make_heartbeat(test_did("a"), 1000));
        monitor.record(make_heartbeat(test_did("b"), 900));

        let now = Timestamp {
            physical_ms: 1150,
            logical: 0,
        };
        let alerts = monitor.check_health(&now);
        // Agent "a" is at 150ms (warning), "b" is at 250ms (critical)
        assert_eq!(alerts.len(), 2);
    }

    #[test]
    fn history_tracking() {
        let mut monitor = HeartbeatMonitor::new();
        let did = test_did("agent1");
        monitor.record(make_heartbeat(did.clone(), 1000));
        monitor.record(make_heartbeat(did.clone(), 2000));
        assert_eq!(monitor.history(&did).unwrap().len(), 2);
    }

    #[test]
    fn status_serde() {
        let statuses = [
            HeartbeatStatus::Queued,
            HeartbeatStatus::Running,
            HeartbeatStatus::Completed,
            HeartbeatStatus::Failed,
            HeartbeatStatus::TimedOut,
        ];
        for s in &statuses {
            let j = serde_json::to_string(s).unwrap();
            let rt: HeartbeatStatus = serde_json::from_str(&j).unwrap();
            assert_eq!(&rt, s);
        }
    }
}
