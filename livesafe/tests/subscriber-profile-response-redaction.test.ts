import { describe, expect, it } from "vitest";

const {
  buildPublicAlertSettingsResponse,
  buildPublicAlertSettingsMutationResponse,
  buildPublicConsentDefaultsResponse,
  buildPublicConsentDefaultsMutationResponse,
  buildPublicEmergencyContactResponse,
  buildPublicSubscriberDeleteAcknowledgement,
  buildPublicSubscriberAllergyWriteResponse,
  buildPublicSubscriberConditionWriteResponse,
  buildPublicSubscriberMedicationWriteResponse,
  buildPublicSubscriberProfileResponse,
  buildPublicSubscriberProfileSummary,
} = require("../server/utils/subscriber-profile-response.js");

describe("subscriber profile response redaction", () => {
  it("returns a bounded subscriber profile without raw subscriber bindings", () => {
    const response = buildPublicSubscriberProfileResponse({
      subscriber: {
        id: 17,
        did: "did:exo:subscriber:private",
        email: "ada@example.com",
        first_name: "Ada",
        last_name: "Lovelace",
        date_of_birth: "1990-12-10",
        blood_type: "O+",
        dnr_status: "not_specified",
        organ_donor: true,
        role: "subscriber",
        email_verified: true,
        alert_sensitivity: "emergency-only",
        phone: "+15551234567",
        phone_verified: true,
        created_at: "2026-06-06T20:00:00.000Z",
      },
      allergies: [
        {
          id: 1,
          subscriber_id: 17,
          allergy: "Peanuts",
          severity: "high",
          created_at: "2026-06-06T20:01:00.000Z",
        },
      ],
      medications: [
        {
          id: 2,
          subscriber_id: 17,
          medication: "Albuterol",
          dosage: "2 puffs",
          frequency: "as needed",
          created_at: "2026-06-06T20:02:00.000Z",
        },
      ],
      conditions: [
        {
          id: 3,
          subscriber_id: 17,
          condition_name: "Asthma",
          diagnosed_date: "2015-01-01",
          notes: "Carries rescue inhaler",
          created_at: "2026-06-06T20:03:00.000Z",
        },
      ],
      emergencyContacts: [
        {
          id: 4,
          subscriber_id: 17,
          name: "Charles Babbage",
          phone: "+15557654321",
          relationship: "Partner",
          created_at: "2026-06-06T20:04:00.000Z",
        },
      ],
    });

    expect(response).toEqual({
      email: "ada@example.com",
      first_name: "Ada",
      last_name: "Lovelace",
      date_of_birth: "1990-12-10",
      blood_type: "O+",
      dnr_status: "not_specified",
      organ_donor: true,
      email_verified: true,
      alert_sensitivity: "emergency-only",
      phone: "+15551234567",
      phone_verified: true,
      allergies: [
        {
          id: 1,
          allergy: "Peanuts",
          severity: "high",
          created_at: "2026-06-06T20:01:00.000Z",
        },
      ],
      medications: [
        {
          id: 2,
          medication: "Albuterol",
          dosage: "2 puffs",
          frequency: "as needed",
          created_at: "2026-06-06T20:02:00.000Z",
        },
      ],
      conditions: [
        {
          id: 3,
          condition_name: "Asthma",
          diagnosed_date: "2015-01-01",
          notes: "Carries rescue inhaler",
          created_at: "2026-06-06T20:03:00.000Z",
        },
      ],
      emergency_contacts: [
        {
          id: 4,
          name: "Charles Babbage",
          phone: "+15557654321",
          relationship: "Partner",
          created_at: "2026-06-06T20:04:00.000Z",
        },
      ],
    });

    expect(response).not.toHaveProperty("id");
    expect(response).not.toHaveProperty("did");
    expect(response).not.toHaveProperty("role");
    expect(response.allergies[0]).not.toHaveProperty("subscriber_id");
    expect(response.medications[0]).not.toHaveProperty("subscriber_id");
    expect(response.conditions[0]).not.toHaveProperty("subscriber_id");
    expect(response.emergency_contacts[0]).not.toHaveProperty("subscriber_id");
  });

  it("returns a bounded profile summary for update acknowledgements", () => {
    const response = buildPublicSubscriberProfileSummary({
      id: 17,
      did: "did:exo:subscriber:private",
      email: "ada@example.com",
      first_name: "Ada",
      last_name: "Lovelace",
      date_of_birth: "1990-12-10",
      blood_type: "O+",
      dnr_status: "not_specified",
      organ_donor: true,
      role: "subscriber",
      email_verified: true,
      created_at: "2026-06-06T20:00:00.000Z",
    });

    expect(response).toEqual({
      email: "ada@example.com",
      first_name: "Ada",
      last_name: "Lovelace",
      date_of_birth: "1990-12-10",
      blood_type: "O+",
      dnr_status: "not_specified",
      organ_donor: true,
      email_verified: true,
      alert_sensitivity: "always",
      phone: null,
      phone_verified: false,
    });
    expect(response).not.toHaveProperty("id");
    expect(response).not.toHaveProperty("did");
    expect(response).not.toHaveProperty("role");
    expect(response).not.toHaveProperty("created_at");
  });

  it("returns a bounded emergency-contact response without subscriber bindings", () => {
    const response = buildPublicEmergencyContactResponse({
      id: 4,
      subscriber_id: 17,
      name: "Charles Babbage",
      phone: "+15557654321",
      relationship: "Partner",
      created_at: "2026-06-06T20:04:00.000Z",
      updated_at: "2026-06-06T20:05:00.000Z",
    });

    expect(response).toEqual({
      id: 4,
      name: "Charles Babbage",
      phone: "+15557654321",
      relationship: "Partner",
      created_at: "2026-06-06T20:04:00.000Z",
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("updated_at");
  });

  it("returns a bounded delete acknowledgement without raw row identifiers", () => {
    const response = buildPublicSubscriberDeleteAcknowledgement({
      message: "Emergency contact removed",
      id: 44,
      subscriber_id: 17,
      deleted_at: "2026-06-07T04:30:00.000Z",
    });

    expect(response).toEqual({
      message: "Emergency contact removed",
    });
    expect(response).not.toHaveProperty("id");
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("deleted_at");
  });

  it("returns a bounded alert-settings response without raw subscriber bindings", () => {
    const response = buildPublicAlertSettingsResponse({
      id: 17,
      did: "did:exo:subscriber:private",
      alert_sensitivity: "emergency-only",
      sms_alerts: true,
      push_alerts: false,
      email_alerts: true,
      updated_at: "2026-06-07T05:58:00.000Z",
    });

    expect(response).toEqual({
      alert_sensitivity: "emergency-only",
      sms_alerts: true,
      push_alerts: false,
      email_alerts: true,
      options: ["always", "emergency-only", "off"],
    });
    expect(response).not.toHaveProperty("id");
    expect(response).not.toHaveProperty("did");
    expect(response).not.toHaveProperty("updated_at");
  });

  it("returns a bounded alert-settings mutation acknowledgement", () => {
    const response = buildPublicAlertSettingsMutationResponse({
      id: 17,
      alert_sensitivity: "off",
      sms_alerts: false,
      push_alerts: false,
      email_alerts: true,
      message: "Alert settings saved",
    });

    expect(response).toEqual({
      alert_sensitivity: "off",
      sms_alerts: false,
      push_alerts: false,
      email_alerts: true,
      message: "Alert settings saved",
    });
    expect(response).not.toHaveProperty("id");
  });

  it("returns a bounded consent-defaults response without raw subscriber bindings", () => {
    const response = buildPublicConsentDefaultsResponse({
      id: 17,
      did: "did:exo:subscriber:private",
      consent_default_scope: "research",
      consent_default_duration_days: 90,
      updated_at: "2026-06-07T05:59:00.000Z",
    });

    expect(response).toEqual({
      default_scope: "research",
      default_duration_days: 90,
      scope_options: ["basic_health", "full_health", "emergency_only", "research"],
      duration_options: [7, 30, 90, 180, 365],
    });
    expect(response).not.toHaveProperty("id");
    expect(response).not.toHaveProperty("did");
    expect(response).not.toHaveProperty("updated_at");
  });

  it("returns a bounded consent-defaults mutation acknowledgement", () => {
    const response = buildPublicConsentDefaultsMutationResponse({
      id: 17,
      consent_default_scope: "full_health",
      consent_default_duration_days: 180,
      message: "Consent defaults updated successfully",
    });

    expect(response).toEqual({
      default_scope: "full_health",
      default_duration_days: 180,
      message: "Consent defaults updated successfully",
    });
    expect(response).not.toHaveProperty("id");
  });

  it("returns a bounded allergy acknowledgement without raw subscriber or claim bindings", () => {
    const response = buildPublicSubscriberAllergyWriteResponse({
      allergy: {
        id: 9,
        subscriber_id: 17,
        allergy: "Peanuts",
        severity: "high",
        created_at: "2026-06-06T20:06:00.000Z",
        updated_at: "2026-06-06T20:06:30.000Z",
      },
      odentityClaim: {
        id: 71,
        subscriber_id: 17,
        claim_type: "allergies_entered",
        dimension: "health_record_completeness",
        points_awarded: "15.00",
        issuer: "livesafe",
        issued_at: "2026-06-06T20:06:00.000Z",
        revoked_at: null,
        created_at: "2026-06-06T20:06:00.000Z",
      },
    });

    expect(response).toEqual({
      id: 9,
      allergy: "Peanuts",
      severity: "high",
      created_at: "2026-06-06T20:06:00.000Z",
      odentity_claim: {
        id: 71,
        claim_type: "allergies_entered",
        dimension: "health_record_completeness",
        points_awarded: 15,
        issuer: "livesafe",
        issued_at: "2026-06-06T20:06:00.000Z",
        revoked_at: null,
      },
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("updated_at");
    expect(response.odentity_claim).not.toHaveProperty("subscriber_id");
    expect(response.odentity_claim).not.toHaveProperty("created_at");
  });

  it("returns a bounded medication acknowledgement without raw subscriber or claim bindings", () => {
    const response = buildPublicSubscriberMedicationWriteResponse({
      medication: {
        id: 10,
        subscriber_id: 17,
        medication: "Albuterol",
        dosage: "2 puffs",
        frequency: "as needed",
        created_at: "2026-06-06T20:07:00.000Z",
      },
      odentityClaim: null,
    });

    expect(response).toEqual({
      id: 10,
      medication: "Albuterol",
      dosage: "2 puffs",
      frequency: "as needed",
      created_at: "2026-06-06T20:07:00.000Z",
      odentity_claim: null,
    });
    expect(response).not.toHaveProperty("subscriber_id");
  });

  it("returns a bounded condition acknowledgement without raw subscriber or claim bindings", () => {
    const response = buildPublicSubscriberConditionWriteResponse({
      condition: {
        id: 11,
        subscriber_id: 17,
        condition_name: "Asthma",
        diagnosed_date: "2015-01-01",
        notes: "Carries rescue inhaler",
        created_at: "2026-06-06T20:08:00.000Z",
      },
      odentityClaim: {
        id: 72,
        subscriber_id: 17,
        claim_type: "conditions_entered",
        dimension: "health_record_completeness",
        points_awarded: "15.00",
        issuer: "livesafe",
        issued_at: "2026-06-06T20:08:00.000Z",
        revoked_at: null,
      },
    });

    expect(response).toEqual({
      id: 11,
      condition_name: "Asthma",
      diagnosed_date: "2015-01-01",
      notes: "Carries rescue inhaler",
      created_at: "2026-06-06T20:08:00.000Z",
      odentity_claim: {
        id: 72,
        claim_type: "conditions_entered",
        dimension: "health_record_completeness",
        points_awarded: 15,
        issuer: "livesafe",
        issued_at: "2026-06-06T20:08:00.000Z",
        revoked_at: null,
      },
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response.odentity_claim).not.toHaveProperty("subscriber_id");
  });
});
