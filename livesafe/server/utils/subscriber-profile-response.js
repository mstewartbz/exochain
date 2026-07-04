const {
  buildPublicOdentityClaimResponse,
} = require("./odentity-claim-response");

function buildPublicSubscriberProfileSummary(subscriber = {}) {
  return {
    email: subscriber.email ?? null,
    first_name: subscriber.first_name ?? null,
    last_name: subscriber.last_name ?? null,
    date_of_birth: subscriber.date_of_birth ?? null,
    blood_type: subscriber.blood_type ?? null,
    dnr_status: subscriber.dnr_status ?? "not_specified",
    organ_donor: subscriber.organ_donor ?? false,
    email_verified: subscriber.email_verified ?? false,
    alert_sensitivity: subscriber.alert_sensitivity ?? "always",
    phone: subscriber.phone ?? null,
    phone_verified: subscriber.phone_verified ?? false,
  };
}

function buildPublicSubscriberAllergyResponse(row = {}) {
  return {
    id: row.id,
    allergy: row.allergy ?? null,
    severity: row.severity ?? null,
    created_at: row.created_at ?? null,
  };
}

function buildPublicSubscriberMedicationResponse(row = {}) {
  return {
    id: row.id,
    medication: row.medication ?? null,
    dosage: row.dosage ?? null,
    frequency: row.frequency ?? null,
    created_at: row.created_at ?? null,
  };
}

function buildPublicSubscriberConditionResponse(row = {}) {
  return {
    id: row.id,
    condition_name: row.condition_name ?? null,
    diagnosed_date: row.diagnosed_date ?? null,
    notes: row.notes ?? null,
    created_at: row.created_at ?? null,
  };
}

function buildPublicEmergencyContactResponse(row = {}) {
  return {
    id: row.id,
    name: row.name ?? null,
    phone: row.phone ?? null,
    relationship: row.relationship ?? null,
    created_at: row.created_at ?? null,
  };
}

function buildPublicSubscriberDeleteAcknowledgement({
  message = "Removed successfully",
} = {}) {
  return {
    message,
  };
}

function buildPublicAlertSettingsResponse(row = {}) {
  return {
    alert_sensitivity: row.alert_sensitivity ?? "always",
    sms_alerts: row.sms_alerts !== false,
    push_alerts: row.push_alerts !== false,
    email_alerts: row.email_alerts !== false,
    options: ["always", "emergency-only", "off"],
  };
}

function buildPublicAlertSettingsMutationResponse({
  message = "Alert settings saved",
  ...row
} = {}) {
  return {
    alert_sensitivity: row.alert_sensitivity ?? "always",
    sms_alerts: row.sms_alerts !== false,
    push_alerts: row.push_alerts !== false,
    email_alerts: row.email_alerts !== false,
    message,
  };
}

function buildPublicConsentDefaultsResponse(row = {}) {
  return {
    default_scope: row.consent_default_scope ?? "basic_health",
    default_duration_days: row.consent_default_duration_days ?? 30,
    scope_options: ["basic_health", "full_health", "emergency_only", "research"],
    duration_options: [7, 30, 90, 180, 365],
  };
}

function buildPublicConsentDefaultsMutationResponse({
  message = "Consent defaults updated successfully",
  ...row
} = {}) {
  return {
    default_scope: row.consent_default_scope ?? "basic_health",
    default_duration_days: row.consent_default_duration_days ?? 30,
    message,
  };
}

function buildPublicSubscriberAllergyWriteResponse({
  allergy = {},
  odentityClaim = null,
} = {}) {
  return {
    ...buildPublicSubscriberAllergyResponse(allergy),
    odentity_claim: odentityClaim
      ? buildPublicOdentityClaimResponse(odentityClaim)
      : null,
  };
}

function buildPublicSubscriberMedicationWriteResponse({
  medication = {},
  odentityClaim = null,
} = {}) {
  return {
    ...buildPublicSubscriberMedicationResponse(medication),
    odentity_claim: odentityClaim
      ? buildPublicOdentityClaimResponse(odentityClaim)
      : null,
  };
}

function buildPublicSubscriberConditionWriteResponse({
  condition = {},
  odentityClaim = null,
} = {}) {
  return {
    ...buildPublicSubscriberConditionResponse(condition),
    odentity_claim: odentityClaim
      ? buildPublicOdentityClaimResponse(odentityClaim)
      : null,
  };
}

function buildPublicSubscriberProfileResponse({
  subscriber = {},
  allergies = [],
  medications = [],
  conditions = [],
  emergencyContacts = [],
} = {}) {
  return {
    ...buildPublicSubscriberProfileSummary(subscriber),
    allergies: allergies.map(buildPublicSubscriberAllergyResponse),
    medications: medications.map(buildPublicSubscriberMedicationResponse),
    conditions: conditions.map(buildPublicSubscriberConditionResponse),
    emergency_contacts: emergencyContacts.map(buildPublicEmergencyContactResponse),
  };
}

module.exports = {
  buildPublicAlertSettingsMutationResponse,
  buildPublicAlertSettingsResponse,
  buildPublicConsentDefaultsMutationResponse,
  buildPublicConsentDefaultsResponse,
  buildPublicEmergencyContactResponse,
  buildPublicSubscriberDeleteAcknowledgement,
  buildPublicSubscriberAllergyWriteResponse,
  buildPublicSubscriberConditionWriteResponse,
  buildPublicSubscriberMedicationWriteResponse,
  buildPublicSubscriberProfileSummary,
  buildPublicSubscriberProfileResponse,
};
