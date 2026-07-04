use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CardLifecycleState {
    Active,
    Expired,
    Replaced,
    Revoked,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PanelRequirement {
    Identity,
    QrActivation,
    MedicalRelease,
    LegacyDirective,
    RightsAssertion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointStatus {
    Current,
    Obsolete,
    Expired,
    Replaced,
    Revoked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardVersion {
    pub card_version_ref: String,
    pub effective_date: String,
    pub lifecycle_state: CardLifecycleState,
    pub revocation_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardPanel {
    pub requirement: PanelRequirement,
    pub enabled: bool,
    pub accepted_by_subscriber: bool,
    pub confirmation_ref: Option<String>,
    pub jurisdiction_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QrPointer {
    pub token_ref: String,
    pub endpoint_ref: String,
    pub endpoint_status: EndpointStatus,
    pub includes_raw_sensitive_payload: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IceCardPacket {
    pub version: CardVersion,
    pub panels: Vec<CardPanel>,
    pub qr_pointer: QrPointer,
    pub has_cut_guides: bool,
    pub has_fold_instructions: bool,
    pub first_fold_instruction_present: bool,
    pub legal_privacy_area_present: bool,
    pub generated_from_preferences: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IceCardPacketDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

const REQUIRED_PANELS: [PanelRequirement; 2] =
    [PanelRequirement::Identity, PanelRequirement::QrActivation];

pub fn evaluate_card_packet(packet: &IceCardPacket) -> IceCardPacketDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();
    let enabled_panels: BTreeSet<PanelRequirement> = packet
        .panels
        .iter()
        .filter(|panel| panel.enabled)
        .map(|panel| panel.requirement)
        .collect();

    if packet.version.card_version_ref.trim().is_empty() {
        reasons
            .insert("ICE card packets require a synthetic shared card-version reference.".into());
        required_evidence.insert(
            "Shared card-version reference for printed and scan-visible packet state.".into(),
        );
    }

    if packet.version.effective_date.trim().is_empty() {
        reasons.insert("ICE card packets require an effective date.".into());
        required_evidence
            .insert("Effective date shared across the printed packet and QR-visible state.".into());
    }

    if packet.version.lifecycle_state == CardLifecycleState::Revoked
        && empty_option(&packet.version.revocation_ref)
    {
        reasons.insert("Revoked ICE card packets require a revocation reference.".into());
        required_evidence
            .insert("Revocation receipt or control reference for revoked cards.".into());
    }

    for required_panel in REQUIRED_PANELS {
        if !enabled_panels.contains(&required_panel) {
            reasons.insert(format!(
                "ICE card packets must include the {} panel.",
                panel_label(required_panel)
            ));
            required_evidence.insert(format!(
                "{} panel configuration for the printable emergency packet.",
                panel_label(required_panel)
            ));
        }
    }

    for panel in packet.panels.iter().filter(|panel| panel.enabled) {
        if panel.requires_explicit_acceptance() && !panel.accepted_by_subscriber {
            reasons.insert(
                "Optional ICE card legal or directive panels require explicit subscriber acceptance."
                    .into(),
            );
            required_evidence.insert(
                "Subscriber acceptance state for medical-release, legacy-directive, and rights-assertion panels."
                    .into(),
            );
        }

        if panel.requires_explicit_acceptance() && empty_option(&panel.confirmation_ref) {
            reasons.insert(
                "Optional ICE card legal or directive panels require a confirmation reference."
                    .into(),
            );
            required_evidence
                .insert("Confirmation reference for each enabled legal or directive panel.".into());
        }

        if panel.requires_explicit_acceptance() && empty_option(&panel.jurisdiction_ref) {
            reasons.insert(
                "Optional ICE card legal or directive panels require a jurisdiction reference."
                    .into(),
            );
            required_evidence
                .insert("Jurisdiction tagging for each enabled legal or directive panel.".into());
        }
    }

    if packet.qr_pointer.token_ref.trim().is_empty()
        || packet.qr_pointer.endpoint_ref.trim().is_empty()
    {
        reasons
            .insert("ICE card QR pointers require synthetic token and endpoint references.".into());
        required_evidence.insert(
            "Synthetic QR token metadata and configuration-backed endpoint reference.".into(),
        );
    }

    if packet.qr_pointer.includes_raw_sensitive_payload {
        reasons.insert(
            "ICE card QR payloads must contain only a retrieval or activation pointer, never raw sensitive data."
                .into(),
        );
        required_evidence.insert(
            "QR payload review proving scan-time retrieval uses a server-side pointer only.".into(),
        );
    }

    if packet.qr_pointer.endpoint_status != EndpointStatus::Current {
        reasons.insert(
            "ICE card QR endpoints must be current generation-time targets; obsolete, expired, replaced, or revoked targets are denied."
                .into(),
        );
        required_evidence.insert(
            "Current configuration-backed endpoint validation at card-generation time.".into(),
        );
    }

    if !packet.has_cut_guides {
        reasons
            .insert("ICE card packets must include cut guides for wallet-card generation.".into());
        required_evidence.insert("Printable cut-guide layout for the wallet-card format.".into());
    }

    if !packet.has_fold_instructions {
        reasons.insert(
            "ICE card packets must include fold-order instructions for printable packets.".into(),
        );
        required_evidence.insert("Packet fold-order instructions in the printable layout.".into());
    }

    if !packet.first_fold_instruction_present {
        reasons.insert(
            "ICE card packets that require folding must include a first-fold instruction.".into(),
        );
        required_evidence.insert("First-fold instruction visible on the printable packet.".into());
    }

    if !packet.legal_privacy_area_present {
        reasons.insert("ICE card packets must include a legal or privacy area.".into());
        required_evidence.insert(
            "Printable legal or privacy area aligned to current scan-time access rules.".into(),
        );
    }

    if !packet.generated_from_preferences {
        reasons.insert(
            "ICE card packets must be generated from account preferences and emergency-card configuration."
                .into(),
        );
        required_evidence.insert(
            "Generation path from account preferences, profile state, and card configuration."
                .into(),
        );
    }

    IceCardPacketDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

impl CardPanel {
    fn requires_explicit_acceptance(&self) -> bool {
        matches!(
            self.requirement,
            PanelRequirement::MedicalRelease
                | PanelRequirement::LegacyDirective
                | PanelRequirement::RightsAssertion
        )
    }
}

fn panel_label(panel: PanelRequirement) -> &'static str {
    match panel {
        PanelRequirement::Identity => "identity",
        PanelRequirement::QrActivation => "QR activation",
        PanelRequirement::MedicalRelease => "medical release",
        PanelRequirement::LegacyDirective => "legacy directive",
        PanelRequirement::RightsAssertion => "rights assertion",
    }
}

fn empty_option(value: &Option<String>) -> bool {
    value
        .as_ref()
        .map(|current| current.trim().is_empty())
        .unwrap_or(true)
}
