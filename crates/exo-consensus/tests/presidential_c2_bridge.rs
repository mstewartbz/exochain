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

//! Presidential Mission C2 bridge: AI-IRB advisories (exo-consensus) must not
//! satisfy Decision Forum two-person ratification. Deterministic/mock only.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use decision_forum::{
    decision_object::{
        ActorKind, DecisionClass, DecisionObject, DecisionObjectInput, Vote, VoteChoice,
    },
    human_gate::{TwoPersonGatePolicy, enforce_two_person_ratification},
};
use exo_consensus::{
    DeliberationSession, DeterministicResponseProvider, DevilAdvocateReview, FinalizationTiming,
    ModelDeliberationResponse, ModelProvider, ModelRole, Panel, RoundExecutionTiming,
    bind_advisory_receipt, require_dissent_receipt,
};
use exo_core::{
    hlc::HybridClock,
    types::{Did, Hash256, Timestamp},
};

fn response(text: &str, claims: &[&str]) -> ModelDeliberationResponse {
    ModelDeliberationResponse {
        position_text: text.to_string(),
        key_claims: claims.iter().map(|c| (*c).to_string()).collect(),
        confidence_bps: 8000,
    }
}

fn neutral_review() -> DevilAdvocateReview {
    DevilAdvocateReview {
        review_text: "No threshold objection; dissent recorded for receipt binding.".into(),
        serious_objection: false,
        reasons: Vec::new(),
    }
}

fn strategic_provider() -> DeterministicResponseProvider {
    let shared = response("proceed with dual-gate ratification", &["dual-gate-required"]);
    DeterministicResponseProvider::new(
        BTreeMap::from([
            ("claude-3-opus".to_string(), shared.clone()),
            ("gpt-4o".to_string(), shared.clone()),
            ("gemini-1.5-pro".to_string(), shared.clone()),
            ("grok-2".to_string(), shared),
        ]),
        BTreeMap::from([("claude-3-opus".to_string(), neutral_review())]),
    )
}

#[test]
fn presidential_airb_advisories_do_not_satisfy_two_person_gate() {
    let panel = Panel::default_panel(DecisionClass::Strategic);
    assert!(
        panel.devil_advocate_model.is_some(),
        "presidential strategic panel must include devil's advocate"
    );
    assert!(
        panel
            .models
            .iter()
            .any(|m| matches!(m.provider, ModelProvider::XAI)),
        "strategic panel must include xAI/Grok seat"
    );

    let mut session = DeliberationSession::new(
        "c2-airb-bridge".into(),
        panel.clone(),
        "Should we ratify this constitutional change?".into(),
        strategic_provider(),
    );
    let round = session
        .execute_round(RoundExecutionTiming {
            submitted_at: Timestamp::new(10_000, 0),
            revealed_at: Timestamp::new(10_000, 1),
        })
        .expect("deterministic AI-IRB round");
    assert!(round.devil_advocate_review.is_some());
    let _result = session
        .finalize(FinalizationTiming {
            completed_at: Timestamp::new(10_001, 0),
        })
        .expect("finalize");

    // Bind per-seat advisory receipts (D9-aligned evidence substrate).
    let mut advisory_receipts = Vec::new();
    for model in &panel.models {
        if !matches!(model.role, ModelRole::Panelist) {
            continue;
        }
        let resp = response("proceed with dual-gate ratification", &["dual-gate-required"]);
        let receipt = bind_advisory_receipt(&model.provider, &model.model_id, "Panelist", &resp)
            .expect("advisory receipt");
        assert!(!receipt.response_hash.is_empty());
        advisory_receipts.push(receipt);
    }
    assert!(advisory_receipts.len() >= 3);

    let da_model = panel.devil_advocate_model.as_deref().expect("DA model");
    let dissent = bind_advisory_receipt(
        &ModelProvider::Anthropic,
        da_model,
        "DevilsAdvocate",
        &response("dissent recorded", &["dissent-present"]),
    )
    .expect("dissent receipt");
    require_dissent_receipt(&dissent.response_hash).expect("dissent id present");

    // AI-IRB evidence alone cannot ratify — Bob+Max dual gate still required.
    let counter = AtomicU64::new(1_000);
    let mut clock = HybridClock::with_wall_clock(move || counter.fetch_add(1, Ordering::Relaxed));
    let policy = TwoPersonGatePolicy::presidential().expect("policy");
    let mut decision = DecisionObject::new(DecisionObjectInput {
        id: uuid::Uuid::from_u128(42),
        title: "C2 bridge constitutional item".into(),
        class: DecisionClass::Constitutional,
        constitutional_hash: Hash256::digest(b"constitution"),
        created_at: clock.now().expect("hlc"),
    })
    .expect("decision");

    decision
        .add_vote(Vote {
            voter_did: Did::new("did:exo:ai-irb-panel").expect("did"),
            choice: VoteChoice::Approve,
            actor_kind: ActorKind::AiAgent {
                delegation_id: "airb".into(),
                ceiling_class: DecisionClass::Operational,
            },
            timestamp: clock.now().expect("hlc"),
            signature_hash: Hash256::digest(b"ai-irb"),
        })
        .expect("ai vote");

    let verified = BTreeSet::new();
    let err = enforce_two_person_ratification(&decision, &verified, &policy)
        .expect_err("AI-IRB cannot satisfy two-person gate");
    assert!(err.to_string().contains("bob-stewart") || err.to_string().contains("mstewartbz"));

    let mut verified = BTreeSet::new();
    verified.insert(policy.principal_a.clone());
    verified.insert(policy.principal_b.clone());
    decision
        .add_vote(Vote {
            voter_did: policy.principal_a.clone(),
            choice: VoteChoice::Approve,
            actor_kind: ActorKind::Human,
            timestamp: clock.now().expect("hlc"),
            signature_hash: Hash256::digest(b"bob"),
        })
        .expect("bob");
    decision
        .add_vote(Vote {
            voter_did: policy.principal_b.clone(),
            choice: VoteChoice::Approve,
            actor_kind: ActorKind::Human,
            timestamp: clock.now().expect("hlc"),
            signature_hash: Hash256::digest(b"max"),
        })
        .expect("max");
    enforce_two_person_ratification(&decision, &verified, &policy)
        .expect("Bob+Max dual attestation satisfies gate");
}
