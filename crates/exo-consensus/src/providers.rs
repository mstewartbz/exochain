// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Feature-gated AI-IRB provider seats.
//!
//! These seats are **deterministic / mock-only** in this crate. They fail closed
//! when unconfigured and never perform live HTTP LLM calls. Enabling a feature
//! does not claim D9 charter enactment or production LLM activation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    error::{ConsensusError, Result},
    panel::ModelProvider,
    round::{DevilAdvocateReview, ModelDeliberationResponse},
};

/// Seat configuration: must be explicitly marked configured with mock responses.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeatConfig {
    pub configured: bool,
    pub positions: BTreeMap<String, ModelDeliberationResponse>,
    pub devil_advocate_reviews: BTreeMap<String, DevilAdvocateReview>,
}

/// A single provider seat that fails closed unless configured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSeat {
    pub provider: ModelProvider,
    pub config: SeatConfig,
}

impl ProviderSeat {
    #[must_use]
    pub fn unconfigured(provider: ModelProvider) -> Self {
        Self {
            provider,
            config: SeatConfig::default(),
        }
    }

    #[must_use]
    pub fn with_mock(
        provider: ModelProvider,
        positions: BTreeMap<String, ModelDeliberationResponse>,
        devil_advocate_reviews: BTreeMap<String, DevilAdvocateReview>,
    ) -> Self {
        Self {
            provider,
            config: SeatConfig {
                configured: true,
                positions,
                devil_advocate_reviews,
            },
        }
    }

    fn provider_label(&self) -> &'static str {
        match self.provider {
            ModelProvider::Anthropic => "anthropic",
            ModelProvider::OpenAI => "openai",
            ModelProvider::Google => "google",
            ModelProvider::XAI => "xai",
            ModelProvider::Other(_) => "other",
        }
    }

    /// Fetch a structured position; fails closed if seat unconfigured or missing.
    pub fn position_for(&self, model_id: &str) -> Result<ModelDeliberationResponse> {
        if !self.config.configured {
            return Err(ConsensusError::ProviderError(format!(
                "{} seat unconfigured",
                self.provider_label()
            )));
        }
        self.config.positions.get(model_id).cloned().ok_or_else(|| {
            ConsensusError::ProviderError(format!(
                "missing structured response for model {model_id} on {} seat",
                self.provider_label()
            ))
        })
    }

    /// Fetch a devil's advocate review; fails closed if seat unconfigured or missing.
    pub fn devil_advocate_review_for(&self, model_id: &str) -> Result<DevilAdvocateReview> {
        if !self.config.configured {
            return Err(ConsensusError::ProviderError(format!(
                "{} seat unconfigured",
                self.provider_label()
            )));
        }
        self.config
            .devil_advocate_reviews
            .get(model_id)
            .cloned()
            .ok_or_else(|| {
                ConsensusError::ProviderError(format!(
                    "missing devil's advocate review for model {model_id} on {} seat",
                    self.provider_label()
                ))
            })
    }
}

/// Presidential AI-IRB cohort seats (active + planned).
#[derive(Debug, Clone)]
pub struct PresidentialCohort {
    pub xai: ProviderSeat,
    pub openai: ProviderSeat,
    pub anthropic: ProviderSeat,
    /// Planned non-voting seats remain unconfigured stubs.
    pub planned: BTreeMap<String, ProviderSeat>,
}

impl PresidentialCohort {
    /// Active seats start unconfigured (fail closed); planned seats are stubs.
    #[must_use]
    pub fn new_fail_closed() -> Self {
        let mut planned = BTreeMap::new();
        for name in [
            "google",
            "meta",
            "deepseek",
            "nvidia",
            "qwen",
            "mistral",
            "amazon",
            "microsoft",
        ] {
            planned.insert(
                name.to_string(),
                ProviderSeat::unconfigured(ModelProvider::Other(name.to_string())),
            );
        }
        Self {
            xai: ProviderSeat::unconfigured(ModelProvider::XAI),
            openai: ProviderSeat::unconfigured(ModelProvider::OpenAI),
            anthropic: ProviderSeat::unconfigured(ModelProvider::Anthropic),
            planned,
        }
    }

    /// Route by provider enum to an active seat.
    pub fn seat_for(&self, provider: &ModelProvider) -> Result<&ProviderSeat> {
        match provider {
            ModelProvider::XAI => Ok(&self.xai),
            ModelProvider::OpenAI => Ok(&self.openai),
            ModelProvider::Anthropic => Ok(&self.anthropic),
            ModelProvider::Google => self.planned.get("google").ok_or_else(|| {
                ConsensusError::ProviderError("google seat unavailable".into())
            }),
            ModelProvider::Other(name) => self.planned.get(name).ok_or_else(|| {
                ConsensusError::ProviderError(format!("{name} seat unavailable"))
            }),
        }
    }
}

/// Advisory receipt binding for presidential brief items (hash-linked, not chat).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryReceipt {
    pub provider: String,
    pub model_id: String,
    pub role: String,
    pub response_hash: String,
}

/// Bind a structured response into an advisory receipt via canonical commitment.
pub fn bind_advisory_receipt(
    provider: &ModelProvider,
    model_id: &str,
    role: &str,
    response: &ModelDeliberationResponse,
) -> Result<AdvisoryReceipt> {
    let hash = crate::commitment::commit_response(response)?;
    let provider_label = match provider {
        ModelProvider::Anthropic => "anthropic",
        ModelProvider::OpenAI => "openai",
        ModelProvider::Google => "google",
        ModelProvider::XAI => "xai",
        ModelProvider::Other(other) => other.as_str(),
    };
    Ok(AdvisoryReceipt {
        provider: provider_label.to_string(),
        model_id: model_id.to_string(),
        role: role.to_string(),
        response_hash: hash.to_string(),
    })
}

/// Fail closed when dissent receipt is missing from an attention item set.
pub fn require_dissent_receipt(dissent_receipt_id: &str) -> Result<()> {
    if dissent_receipt_id.trim().is_empty() {
        return Err(ConsensusError::ProviderError(
            "presidential brief item missing dissent_receipt_id".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::ModelRole;

    fn sample_response() -> ModelDeliberationResponse {
        ModelDeliberationResponse {
            position_text: "approve with conditions".into(),
            key_claims: vec!["claim-a".into()],
            confidence_bps: 7000,
        }
    }

    #[test]
    fn unconfigured_seat_fails_closed() {
        let seat = ProviderSeat::unconfigured(ModelProvider::XAI);
        let err = seat.position_for("grok-2").expect_err("unconfigured");
        assert!(err.to_string().contains("xai seat unconfigured"));
    }

    #[test]
    fn configured_mock_seat_returns_response() {
        let mut positions = BTreeMap::new();
        positions.insert("grok-2".into(), sample_response());
        let seat = ProviderSeat::with_mock(ModelProvider::XAI, positions, BTreeMap::new());
        let got = seat.position_for("grok-2").expect("mock");
        assert_eq!(got.key_claims, vec!["claim-a".to_string()]);
    }

    #[test]
    fn planned_seats_are_non_voting_stubs() {
        let cohort = PresidentialCohort::new_fail_closed();
        assert!(!cohort.planned["deepseek"].config.configured);
        let err = cohort
            .seat_for(&ModelProvider::Other("deepseek".into()))
            .expect("stub exists")
            .position_for("any")
            .expect_err("unconfigured");
        assert!(err.to_string().contains("unconfigured"));
    }

    #[test]
    fn advisory_receipt_binds_response_hash() {
        let response = sample_response();
        let receipt = bind_advisory_receipt(
            &ModelProvider::OpenAI,
            "gpt-4o",
            &format!("{:?}", ModelRole::DevilsAdvocate),
            &response,
        )
        .expect("bind");
        assert_eq!(receipt.provider, "openai");
        assert!(!receipt.response_hash.is_empty());
    }

    #[test]
    fn missing_dissent_receipt_fails_closed() {
        assert!(require_dissent_receipt("").is_err());
        assert!(require_dissent_receipt("dissent-1").is_ok());
    }

    #[test]
    fn provider_modules_have_no_live_http_clients() {
        let source = include_str!("providers.rs");
        let production = source.split("#[cfg(test)]").next().expect("prod");
        for needle in ["reqwest", "ureq", "hyper::", "SystemTime::now"] {
            assert!(
                !production.contains(needle),
                "providers must not include live HTTP/wall-clock ({needle})"
            );
        }
    }
}
