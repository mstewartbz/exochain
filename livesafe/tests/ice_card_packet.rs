use livesafe::ice_card_packet::{
    CardLifecycleState, CardPanel, CardVersion, EndpointStatus, IceCardPacket, PanelRequirement,
    QrPointer, evaluate_card_packet,
};

fn valid_packet() -> IceCardPacket {
    IceCardPacket {
        version: CardVersion {
            card_version_ref: "card:version:2026-05-25".into(),
            effective_date: "2026-05-25".into(),
            lifecycle_state: CardLifecycleState::Active,
            revocation_ref: None,
        },
        panels: vec![
            CardPanel {
                requirement: PanelRequirement::Identity,
                enabled: true,
                accepted_by_subscriber: true,
                confirmation_ref: None,
                jurisdiction_ref: None,
            },
            CardPanel {
                requirement: PanelRequirement::QrActivation,
                enabled: true,
                accepted_by_subscriber: true,
                confirmation_ref: None,
                jurisdiction_ref: None,
            },
            CardPanel {
                requirement: PanelRequirement::MedicalRelease,
                enabled: true,
                accepted_by_subscriber: true,
                confirmation_ref: Some("confirm:medical-release".into()),
                jurisdiction_ref: Some("jurisdiction:us-nc".into()),
            },
            CardPanel {
                requirement: PanelRequirement::LegacyDirective,
                enabled: true,
                accepted_by_subscriber: true,
                confirmation_ref: Some("confirm:legacy".into()),
                jurisdiction_ref: Some("jurisdiction:us-nc".into()),
            },
            CardPanel {
                requirement: PanelRequirement::RightsAssertion,
                enabled: true,
                accepted_by_subscriber: true,
                confirmation_ref: Some("confirm:rights".into()),
                jurisdiction_ref: Some("jurisdiction:us-nc".into()),
            },
        ],
        qr_pointer: QrPointer {
            token_ref: "qr:token:synthetic".into(),
            endpoint_ref: "config:endpoint:responder".into(),
            endpoint_status: EndpointStatus::Current,
            includes_raw_sensitive_payload: false,
        },
        has_cut_guides: true,
        has_fold_instructions: true,
        first_fold_instruction_present: true,
        legal_privacy_area_present: true,
        generated_from_preferences: true,
    }
}

#[test]
fn packet_requires_safe_qr_pointer_and_current_endpoint() {
    let mut packet = valid_packet();
    packet.qr_pointer.includes_raw_sensitive_payload = true;
    packet.qr_pointer.endpoint_status = EndpointStatus::Obsolete;

    let decision = evaluate_card_packet(&packet);

    assert!(!decision.allowed);
    assert!(decision.reasons.contains(
        &"ICE card QR payloads must contain only a retrieval or activation pointer, never raw sensitive data.".into()
    ));
    assert!(decision.reasons.contains(
        &"ICE card QR endpoints must be current generation-time targets; obsolete, expired, replaced, or revoked targets are denied.".into()
    ));
}

#[test]
fn packet_requires_fold_guides_and_generation_from_preferences() {
    let mut packet = valid_packet();
    packet.has_cut_guides = false;
    packet.has_fold_instructions = false;
    packet.first_fold_instruction_present = false;
    packet.generated_from_preferences = false;

    let decision = evaluate_card_packet(&packet);

    assert!(!decision.allowed);
    assert!(
        decision.reasons.contains(
            &"ICE card packets must include cut guides for wallet-card generation.".into()
        )
    );
    assert!(decision.reasons.contains(
        &"ICE card packets must include fold-order instructions for printable packets.".into()
    ));
    assert!(decision.reasons.contains(
        &"ICE card packets that require folding must include a first-fold instruction.".into()
    ));
    assert!(decision.reasons.contains(
        &"ICE card packets must be generated from account preferences and emergency-card configuration.".into()
    ));
}

#[test]
fn optional_legal_and_directive_panels_require_acceptance_confirmation_and_jurisdiction() {
    let mut packet = valid_packet();
    for panel in &mut packet.panels {
        if panel.requirement != PanelRequirement::Identity
            && panel.requirement != PanelRequirement::QrActivation
        {
            panel.accepted_by_subscriber = false;
            panel.confirmation_ref = None;
            panel.jurisdiction_ref = None;
        }
    }

    let decision = evaluate_card_packet(&packet);

    assert!(!decision.allowed);
    assert!(
        decision.reasons.contains(
            &"Optional ICE card legal or directive panels require explicit subscriber acceptance."
                .into()
        )
    );
    assert!(decision.reasons.contains(
        &"Optional ICE card legal or directive panels require a confirmation reference.".into()
    ));
    assert!(decision.reasons.contains(
        &"Optional ICE card legal or directive panels require a jurisdiction reference.".into()
    ));
}

#[test]
fn valid_packet_keeps_shared_version_state_and_required_panels() {
    let packet = valid_packet();

    let decision = evaluate_card_packet(&packet);

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.reasons, Vec::<String>::new());
    assert_eq!(decision.required_evidence, Vec::<String>::new());
}
