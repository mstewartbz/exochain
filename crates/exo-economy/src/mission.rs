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

//! Mission economics primitives.

use exo_core::{Did, Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    error::EconomyError,
    types::MicroExo,
    value_contribution::{require_non_empty, require_nonzero_hash, require_nonzero_timestamp},
};

pub const MISSION_HASH_DOMAIN: &str = "exo.economy.mission.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MissionType {
    ClientServices,
    SoftwareSale,
    PlatformBuild,
    Implementation,
    GovernanceReview,
    GtmCampaign,
    PartnerChannel,
    IpArtifact,
    UpstreamRecognition,
    AgentWorkflow,
    ApexVelocityCatalystClientServices,
    ApexVelocityCatalystSoftwareChannel,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MissionStatus {
    Draft,
    Active,
    Suspended,
    Completed,
    Settled,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionPurpose {
    pub problem: String,
    pub served_party: String,
    pub promised_outcome: String,
    pub expected_value: String,
    pub risk_surface: String,
    pub proof_required: String,
    pub success_condition: String,
}

impl MissionPurpose {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_non_empty(&self.problem, "mission_purpose.problem")?;
        require_non_empty(&self.served_party, "mission_purpose.served_party")?;
        require_non_empty(&self.promised_outcome, "mission_purpose.promised_outcome")?;
        require_non_empty(&self.expected_value, "mission_purpose.expected_value")?;
        require_non_empty(&self.risk_surface, "mission_purpose.risk_surface")?;
        require_non_empty(&self.proof_required, "mission_purpose.proof_required")?;
        require_non_empty(&self.success_condition, "mission_purpose.success_condition")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mission {
    pub mission_id: Hash256,
    pub name: String,
    pub mission_type: MissionType,
    pub owner_did: Did,
    pub principal_did: Did,
    pub purpose: MissionPurpose,
    pub related_platforms: Vec<String>,
    pub expected_value_micro_exo: Option<MicroExo>,
    pub ruleset_id: Hash256,
    pub status: MissionStatus,
    pub created_at: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct MissionHashPayload<'a> {
    domain: &'static str,
    name: &'a str,
    mission_type: MissionType,
    owner_did: &'a Did,
    principal_did: &'a Did,
    purpose: &'a MissionPurpose,
    related_platforms: &'a [String],
    expected_value_micro_exo: Option<MicroExo>,
    ruleset_id: &'a Hash256,
    status: MissionStatus,
    created_at: &'a Timestamp,
}

impl Mission {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_non_empty(&self.name, "mission.name")?;
        self.purpose.validate()?;
        for platform in &self.related_platforms {
            require_non_empty(platform, "mission.related_platforms")?;
        }
        require_nonzero_hash(self.ruleset_id, "mission.ruleset_id")?;
        require_nonzero_timestamp(self.created_at, "mission.created_at")?;
        Ok(())
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&MissionHashPayload {
            domain: MISSION_HASH_DOMAIN,
            name: &self.name,
            mission_type: self.mission_type,
            owner_did: &self.owner_did,
            principal_did: &self.principal_did,
            purpose: &self.purpose,
            related_platforms: &self.related_platforms,
            expected_value_micro_exo: self.expected_value_micro_exo,
            ruleset_id: &self.ruleset_id,
            status: self.status,
            created_at: &self.created_at,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        self.related_platforms.sort();
        self.related_platforms.dedup();
        let hash = self.recompute_content_hash()?;
        self.mission_id = hash;
        self.content_hash = hash;
        Ok(self)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::value_contribution::test_support::{did, h, ts};

    pub fn purpose() -> MissionPurpose {
        MissionPurpose {
            problem: "Governance-ready AI adoption".into(),
            served_party: "client".into(),
            promised_outcome: "auditable implementation".into(),
            expected_value: "reduced risk and useful deployment".into(),
            risk_surface: "AI governance".into(),
            proof_required: "receipts and settlement lines".into(),
            success_condition: "accepted mission outcome".into(),
        }
    }

    pub fn sample_mission() -> Mission {
        Mission {
            mission_id: Hash256::ZERO,
            name: "Apex Velocity Catalyst client services".into(),
            mission_type: MissionType::ApexVelocityCatalystClientServices,
            owner_did: did("owner"),
            principal_did: did("principal"),
            purpose: purpose(),
            related_platforms: vec!["EXOCHAIN".into(), "CommandBase".into()],
            expected_value_micro_exo: Some(150_000_000_000),
            ruleset_id: h(0x21),
            status: MissionStatus::Active,
            created_at: ts(1_100),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;

    #[test]
    fn mission_hash_stable() {
        let mission = sample_mission().anchor().unwrap();
        let again = sample_mission().anchor().unwrap();
        assert_eq!(mission.mission_id, again.mission_id);
        assert_eq!(mission.content_hash, again.content_hash);
    }

    #[test]
    fn mission_hash_changes_with_name() {
        let mission = sample_mission().anchor().unwrap();
        let mut other = sample_mission();
        other.name = "different".into();
        let other = other.anchor().unwrap();
        assert_ne!(mission.content_hash, other.content_hash);
    }
}
