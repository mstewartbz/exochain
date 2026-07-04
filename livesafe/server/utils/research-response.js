function buildResearchOptInResponse(record) {
  const optedIn = Boolean(record?.opted_in);

  return {
    opted_in: optedIn,
    opt_in_at: record?.opt_in_at ?? null,
    opt_out_at: record?.opt_out_at ?? null,
    consent_scope: record?.consent_scope ?? 'de_identified_trial_matching',
    bridge_status: optedIn ? 'subscriber_opted_in' : 'subscriber_not_opted_in',
    policy: 'CyberMedica_Bridge_v1',
  };
}

function buildResearchOptInMutationResponse({ record, message }) {
  return {
    success: true,
    message,
    data: buildResearchOptInResponse(record),
  };
}

function buildResearchAuditEventResponse(event) {
  return {
    id: event.id,
    event_type: event.event_type,
    scope: event.scope,
    created_at: event.created_at,
    event_summary: `${event.event_type} recorded`,
  };
}

function buildResearchAuditTrailResponse(events) {
  return events.map(buildResearchAuditEventResponse);
}

function buildResearchTrialConsentResponse(consent) {
  return {
    trial_id: consent.trial_id,
    trial_title: consent.trial_title,
    status: consent.status,
    consented_at: consent.consented_at,
    withdrawn_at: consent.withdrawn_at ?? null,
    trial_matching_status: consent.status === 'active' ? 'enrolled' : 'withdrawn',
  };
}

function buildResearchTrialConsentListResponse(consents) {
  return consents.map(buildResearchTrialConsentResponse);
}

function buildResearchTrialConsentMutationResponse({
  consent,
  message,
  trialId,
  trialTitle,
  status,
  changedAtKey,
  changedAt,
}) {
  const consentResponse = buildResearchTrialConsentResponse({
    ...consent,
    status: status === 'enrolled' ? 'active' : 'withdrawn',
    consented_at:
      changedAtKey === 'consented_at' ? changedAt : consent.consented_at,
    withdrawn_at:
      changedAtKey === 'withdrawn_at'
        ? changedAt
        : (consent.withdrawn_at ?? null),
  });

  return {
    success: true,
    message,
    trial_id: trialId,
    trial_title: trialTitle,
    [changedAtKey]: changedAt,
    consent: {
      ...consentResponse,
      trial_matching_status: status,
    },
  };
}

function buildResearchSubscriberTrialMatchResponse({
  matchedTrials,
  totalChecked,
  totalEligible,
  checkedAt,
}) {
  return {
    trials: matchedTrials.map((trial) => ({
      trial_id: trial.trial_id,
      title: trial.title,
      sponsor: trial.sponsor,
      phase: trial.phase,
      description: trial.description,
      eligible: trial.eligible,
      checked_at: trial.checked_at,
    })),
    total_checked: totalChecked,
    total_eligible: totalEligible,
    phi_exposed: false,
    zk_scheme: 'groth16-simulated',
    checked_at: checkedAt,
    policy: 'CyberMedica_Bridge_v1',
  };
}

module.exports = {
  buildResearchAuditEventResponse,
  buildResearchAuditTrailResponse,
  buildResearchOptInMutationResponse,
  buildResearchOptInResponse,
  buildResearchSubscriberTrialMatchResponse,
  buildResearchTrialConsentListResponse,
  buildResearchTrialConsentMutationResponse,
  buildResearchTrialConsentResponse,
};
