//! Telegram adjutant — agentic chief-of-staff for human operator oversight.
//!
//! A Telegram bot that acts as the operator's adjutant, presenting clear
//! choices via inline keyboard buttons for every material action.  The
//! bot receives alerts from the sentinel system and forwards them with
//! actionable options.
//!
//! ## Configuration
//!
//! Set environment variables:
//! - `TELEGRAM_BOT_TOKEN` — bot token from @BotFather
//! - `TELEGRAM_CHAT_ID`   — admin chat/group ID
//!
//! If either is unset, the adjutant logs a notice and does not start.
//!
//! ## Interactions
//!
//! | Command / Callback | Action |
//! |---------------------|--------|
//! | `/status`           | Node status with action buttons |
//! | `/receipts`         | Recent trust receipts |
//! | `/challenges`       | Active challenges with review/dismiss |
//! | `/sentinels`        | Sentinel health dashboard |
//! | Inline buttons      | Direct actions (review, dismiss, ack) |

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::challenges::SharedChallengeStore;
use crate::reactor::SharedReactorState;
use crate::sentinels::{AlertReceiver, SentinelAlert, SharedSentinelState};
use crate::store::SqliteDagStore;

// ---------------------------------------------------------------------------
// Telegram API types (minimal subset)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct SendMessageRequest {
    chat_id: String,
    text: String,
    parse_mode: Option<String>,
    reply_markup: Option<InlineKeyboardMarkup>,
}

#[derive(Debug, Serialize)]
struct InlineKeyboardMarkup {
    inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Serialize)]
struct InlineKeyboardButton {
    text: String,
    callback_data: String,
}

#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    #[allow(dead_code)]
    ok: bool,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Update {
    update_id: i64,
    message: Option<TgMessage>,
    callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Deserialize)]
struct TgMessage {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    id: String,
    data: Option<String>,
}

// ---------------------------------------------------------------------------
// Adjutant
// ---------------------------------------------------------------------------

/// Configuration for the Telegram adjutant.
#[derive(Debug, Clone)]
pub struct AdjutantConfig {
    pub bot_token: String,
    pub chat_id: String,
}

impl AdjutantConfig {
    /// Load from environment variables.  Returns `None` if not configured.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID").ok()?;
        if token.is_empty() || chat_id.is_empty() {
            return None;
        }
        Some(Self {
            bot_token: token,
            chat_id,
        })
    }
}

/// The Telegram adjutant — chief-of-staff bot.
pub struct Adjutant {
    config: AdjutantConfig,
    client: reqwest::Client,
    last_update_id: i64,
}

impl Adjutant {
    /// Create a new adjutant.
    #[must_use]
    pub fn new(config: AdjutantConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            last_update_id: 0,
        }
    }

    /// Telegram Bot API base URL.
    fn api_url(&self, method: &str) -> String {
        format!(
            "https://api.telegram.org/bot{}/{}",
            self.config.bot_token, method
        )
    }

    /// Send a text message with optional inline keyboard.
    pub async fn send_message(
        &self,
        text: &str,
        keyboard: Option<Vec<Vec<(&str, &str)>>>,
    ) -> Result<(), String> {
        let reply_markup = keyboard.map(|rows| InlineKeyboardMarkup {
            inline_keyboard: rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|(label, data)| InlineKeyboardButton {
                            text: label.to_string(),
                            callback_data: data.to_string(),
                        })
                        .collect()
                })
                .collect(),
        });

        let body = SendMessageRequest {
            chat_id: self.config.chat_id.clone(),
            text: text.to_string(),
            parse_mode: Some("HTML".to_string()),
            reply_markup,
        };

        self.client
            .post(&self.api_url("sendMessage"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("telegram send: {e}"))?;

        Ok(())
    }

    /// Poll for new updates (long-poll, 10s timeout).
    pub async fn poll_updates(&mut self) -> Vec<Update> {
        let url = format!(
            "{}?offset={}&timeout=10",
            self.api_url("getUpdates"),
            self.last_update_id + 1
        );

        let resp = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(err = %e, "Telegram poll failed");
                return Vec::new();
            }
        };

        let parsed: TelegramResponse<Vec<Update>> = match resp.json().await {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!(err = %e, "Telegram parse failed");
                return Vec::new();
            }
        };

        if let Some(updates) = parsed.result {
            if let Some(last) = updates.last() {
                self.last_update_id = last.update_id;
            }
            updates
        } else {
            Vec::new()
        }
    }

    /// Acknowledge a callback query (removes the "loading" indicator).
    pub async fn answer_callback(&self, callback_id: &str) {
        let _ = self
            .client
            .post(&self.api_url("answerCallbackQuery"))
            .json(&serde_json::json!({ "callback_query_id": callback_id }))
            .send()
            .await;
    }

    /// Send a sentinel alert with action buttons.
    pub async fn send_alert(&self, alert: &SentinelAlert) {
        let emoji = match alert.severity {
            crate::sentinels::Severity::Critical => "\u{1f6a8}", // 🚨
            crate::sentinels::Severity::Warning => "\u{26a0}\u{fe0f}", // ⚠️
            crate::sentinels::Severity::Info => "\u{2139}\u{fe0f}", // ℹ️
        };

        let text = format!(
            "{emoji} <b>SENTINEL: {}</b>\n{}\n\nSeverity: {:?}",
            alert.check, alert.message, alert.severity
        );

        let keyboard = vec![vec![
            ("\u{2705} Acknowledge", "sentinel:ack"),
            ("\u{1f50d} Details", "cmd:sentinels"),
        ]];

        let _ = self.send_message(&text, Some(keyboard)).await;
    }
}

// ---------------------------------------------------------------------------
// Message builders
// ---------------------------------------------------------------------------

/// Build the /status response.
pub fn build_status_message(
    reactor: &SharedReactorState,
    store: &Arc<Mutex<SqliteDagStore>>,
) -> (String, Vec<Vec<(&'static str, &'static str)>>) {
    let (round, height, validator_count, is_validator) = {
        let s = reactor.lock().expect("reactor lock");
        (
            s.consensus.current_round,
            s.consensus.committed.len() as u64,
            s.consensus.config.validators.len(),
            s.is_validator,
        )
    };

    let store_height = {
        let st = store.lock().expect("store lock");
        st.committed_height_value()
    };

    let role = if is_validator { "Validator" } else { "Observer" };

    let text = format!(
        "\u{1f4ca} <b>EXOCHAIN Node Status</b>\n\
         \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\n\
         Round: <code>{round}</code> | Height: <code>{height}</code>\n\
         Store Height: <code>{store_height}</code>\n\
         Validators: <code>{validator_count}</code> | Role: {role}",
    );

    let keyboard = vec![
        vec![
            ("\u{1f4dd} Receipts", "cmd:receipts"),
            ("\u{26a0}\u{fe0f} Challenges", "cmd:challenges"),
        ],
        vec![
            ("\u{1f6e1}\u{fe0f} Sentinels", "cmd:sentinels"),
            ("\u{1f504} Refresh", "cmd:status"),
        ],
    ];

    (text, keyboard)
}

/// Build the /sentinels response.
pub fn build_sentinels_message(
    sentinel_state: &SharedSentinelState,
) -> (String, Vec<Vec<(&'static str, &'static str)>>) {
    let statuses = sentinel_state.lock().expect("sentinel lock");

    let mut text = String::from("\u{1f6e1}\u{fe0f} <b>Sentinel Status</b>\n\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\n");

    if statuses.is_empty() {
        text.push_str("No sentinel data yet — checks run every 30s.");
    } else {
        for s in statuses.iter() {
            let icon = if s.healthy { "\u{2705}" } else { "\u{274c}" };
            text.push_str(&format!("{icon} <b>{}</b>: {}\n", s.check, s.message));
        }
    }

    let keyboard = vec![vec![
        ("\u{1f504} Refresh", "cmd:sentinels"),
        ("\u{1f4ca} Status", "cmd:status"),
    ]];

    (text, keyboard)
}

/// Build the /challenges response.
pub fn build_challenges_message(
    challenge_store: &SharedChallengeStore,
) -> (String, Vec<Vec<(&'static str, &'static str)>>) {
    let st = challenge_store.lock().expect("challenge lock");
    let holds = st.list();

    let mut text = String::from("\u{26a0}\u{fe0f} <b>Active Challenges</b>\n\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\n");

    if holds.is_empty() {
        text.push_str("No active challenges.");
    } else {
        for h in holds {
            text.push_str(&format!(
                "\u{2022} <code>{}</code>\n  Ground: {}\n  Status: {:?}\n\n",
                &h.id.to_string()[..8],
                h.ground,
                h.status
            ));
        }
    }

    let keyboard = vec![vec![
        ("\u{1f504} Refresh", "cmd:challenges"),
        ("\u{1f4ca} Status", "cmd:status"),
    ]];

    (text, keyboard)
}

// ---------------------------------------------------------------------------
// Main adjutant loop
// ---------------------------------------------------------------------------

/// Run the Telegram adjutant as a background Tokio task.
///
/// Handles:
/// 1. Incoming commands from the operator (`/status`, `/receipts`, etc.)
/// 2. Callback queries from inline keyboard buttons
/// 3. Sentinel alerts forwarded from the alert channel
#[allow(clippy::too_many_arguments)]
pub async fn run_adjutant(
    mut adjutant: Adjutant,
    mut alert_rx: AlertReceiver,
    reactor: SharedReactorState,
    store: Arc<Mutex<SqliteDagStore>>,
    challenge_store: SharedChallengeStore,
    sentinel_state: SharedSentinelState,
) {
    // Announce startup.
    let _ = adjutant
        .send_message(
            "\u{1f916} <b>EXOCHAIN Adjutant Online</b>\n\nType /status for node overview.",
            Some(vec![vec![
                ("\u{1f4ca} Status", "cmd:status"),
                ("\u{1f6e1}\u{fe0f} Sentinels", "cmd:sentinels"),
            ]]),
        )
        .await;

    loop {
        tokio::select! {
            // Forward sentinel alerts to Telegram.
            Some(alert) = alert_rx.recv() => {
                adjutant.send_alert(&alert).await;
            }

            // Poll for Telegram updates.
            updates = adjutant.poll_updates() => {
                for update in updates {
                    // Handle text commands.
                    if let Some(msg) = &update.message {
                        if let Some(text) = &msg.text {
                            handle_command(
                                &adjutant,
                                text,
                                &reactor,
                                &store,
                                &challenge_store,
                                &sentinel_state,
                            )
                            .await;
                        }
                    }

                    // Handle callback queries (button presses).
                    if let Some(cb) = &update.callback_query {
                        adjutant.answer_callback(&cb.id).await;
                        if let Some(data) = &cb.data {
                            handle_callback(
                                &adjutant,
                                data,
                                &reactor,
                                &store,
                                &challenge_store,
                                &sentinel_state,
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }
}

async fn handle_command(
    adjutant: &Adjutant,
    text: &str,
    reactor: &SharedReactorState,
    store: &Arc<Mutex<SqliteDagStore>>,
    challenge_store: &SharedChallengeStore,
    sentinel_state: &SharedSentinelState,
) {
    let cmd = text.trim().split_whitespace().next().unwrap_or("");
    match cmd {
        "/status" | "/start" => {
            let (msg, kb) = build_status_message(reactor, store);
            let _ = adjutant.send_message(&msg, Some(kb)).await;
        }
        "/sentinels" => {
            let (msg, kb) = build_sentinels_message(sentinel_state);
            let _ = adjutant.send_message(&msg, Some(kb)).await;
        }
        "/challenges" => {
            let (msg, kb) = build_challenges_message(challenge_store);
            let _ = adjutant.send_message(&msg, Some(kb)).await;
        }
        "/help" => {
            let _ = adjutant
                .send_message(
                    "\u{1f4d6} <b>Commands</b>\n\
                     /status — Node overview\n\
                     /sentinels — Health checks\n\
                     /challenges — Active disputes\n\
                     /help — This message",
                    None,
                )
                .await;
        }
        _ => {}
    }
}

async fn handle_callback(
    adjutant: &Adjutant,
    data: &str,
    reactor: &SharedReactorState,
    store: &Arc<Mutex<SqliteDagStore>>,
    challenge_store: &SharedChallengeStore,
    sentinel_state: &SharedSentinelState,
) {
    match data {
        "cmd:status" => {
            let (msg, kb) = build_status_message(reactor, store);
            let _ = adjutant.send_message(&msg, Some(kb)).await;
        }
        "cmd:sentinels" => {
            let (msg, kb) = build_sentinels_message(sentinel_state);
            let _ = adjutant.send_message(&msg, Some(kb)).await;
        }
        "cmd:challenges" => {
            let (msg, kb) = build_challenges_message(challenge_store);
            let _ = adjutant.send_message(&msg, Some(kb)).await;
        }
        "sentinel:ack" => {
            let _ = adjutant
                .send_message("\u{2705} Alert acknowledged.", None)
                .await;
        }
        _ => {
            tracing::debug!(callback_data = %data, "Unknown callback");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::BTreeSet;

    use exo_core::types::{Did, Signature};

    use super::*;
    use crate::challenges::ChallengeStore;
    use crate::reactor::{ReactorConfig, create_reactor_state};
    use crate::sentinels::{SentinelCheck, SentinelStatus};
    use crate::store::SqliteDagStore;

    fn make_sign_fn() -> Arc<dyn Fn(&[u8]) -> Signature + Send + Sync> {
        Arc::new(|data: &[u8]| {
            let h = blake3::hash(data);
            let mut sig = [0u8; 64];
            sig[..32].copy_from_slice(h.as_bytes());
            Signature::from_bytes(sig)
        })
    }

    fn test_reactor() -> SharedReactorState {
        let validators: BTreeSet<Did> = (0..4)
            .map(|i| Did::new(&format!("did:exo:v{i}")).unwrap())
            .collect();
        let config = ReactorConfig {
            node_did: Did::new("did:exo:v0").unwrap(),
            is_validator: true,
            validators,
            round_timeout_ms: 5000,
        };
        create_reactor_state(&config, make_sign_fn(), None)
    }

    #[test]
    fn config_from_env_returns_none_when_unset() {
        // Env vars are not set in test environment.
        assert!(AdjutantConfig::from_env().is_none());
    }

    #[test]
    fn status_message_contains_key_metrics() {
        let reactor = test_reactor();
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(SqliteDagStore::open(dir.path()).unwrap()));

        let (text, keyboard) = build_status_message(&reactor, &store);
        assert!(text.contains("Round:"));
        assert!(text.contains("Height:"));
        assert!(text.contains("Validators:"));
        assert!(text.contains("Validator")); // role
        assert!(!keyboard.is_empty());
    }

    #[test]
    fn sentinels_message_shows_statuses() {
        let state: SharedSentinelState = Arc::new(Mutex::new(vec![
            SentinelStatus {
                check: SentinelCheck::Liveness,
                healthy: true,
                message: "ok".into(),
                last_run_ms: 0,
            },
        ]));
        let (text, _) = build_sentinels_message(&state);
        assert!(text.contains("Liveness"));
        assert!(text.contains("ok"));
    }

    #[test]
    fn challenges_message_empty() {
        let store: SharedChallengeStore =
            Arc::new(Mutex::new(ChallengeStore::new()));
        let (text, _) = build_challenges_message(&store);
        assert!(text.contains("No active challenges"));
    }

    #[test]
    fn challenges_message_with_hold() {
        use exo_escalation::challenge::{self, SybilChallengeGround};

        let store: SharedChallengeStore =
            Arc::new(Mutex::new(ChallengeStore::new()));
        {
            let mut st = store.lock().unwrap();
            let hold = challenge::admit_challenge(
                &[1u8; 32],
                SybilChallengeGround::QuorumContamination,
                exo_core::types::Timestamp::new(1000, 0),
            );
            st.insert(hold);
        }
        let (text, _) = build_challenges_message(&store);
        assert!(text.contains("QuorumContamination"));
        assert!(text.contains("PauseEligible"));
    }
}
