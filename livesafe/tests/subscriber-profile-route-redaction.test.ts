import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("subscriber profile route redaction wiring", () => {
  it("routes subscriber profile reads and writes through a bounded helper", () => {
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );
    const profileGetStart = subscriberRoute.indexOf("router.get('/profile'");
    const profilePutStart = subscriberRoute.indexOf("router.put('/profile'");
    const allergiesStart = subscriberRoute.indexOf("router.post('/profile/allergies'");
    const profileGetBlock = subscriberRoute.slice(profileGetStart, profilePutStart);
    const profilePutBlock = subscriberRoute.slice(profilePutStart, allergiesStart);

    expect(subscriberRoute).toContain("buildPublicSubscriberProfileResponse({");
    expect(subscriberRoute).toContain(
      "res.json(buildPublicSubscriberProfileSummary(result.rows[0]));",
    );
    expect(profileGetBlock).not.toContain("res.json({\n      ...subscriber,");
    expect(profileGetBlock).not.toContain("res.json(subResult.rows[0]);");
    expect(profileGetBlock).not.toContain("res.json({ ...subscriber");
    expect(profilePutBlock).not.toContain("res.json(result.rows[0]);");
  });

  it("routes emergency-contact create and update responses through the bounded helper", () => {
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );
    const contactPostStart = subscriberRoute.indexOf(
      "router.post('/profile/emergency-contacts'",
    );
    const contactPutStart = subscriberRoute.indexOf(
      "router.put('/profile/emergency-contacts/:id'",
    );
    const contactDeleteStart = subscriberRoute.indexOf(
      "router.delete('/profile/emergency-contacts/:id'",
    );
    const contactPostBlock = subscriberRoute.slice(contactPostStart, contactPutStart);
    const contactPutBlock = subscriberRoute.slice(contactPutStart, contactDeleteStart);

    expect(subscriberRoute).toContain("buildPublicEmergencyContactResponse");
    expect(contactPostBlock).toContain(
      "res.status(201).json(buildPublicEmergencyContactResponse(result.rows[0]));",
    );
    expect(contactPutBlock).toContain(
      "res.json(buildPublicEmergencyContactResponse(result.rows[0]));",
    );
    expect(contactPostBlock).not.toContain("res.status(201).json(result.rows[0]);");
    expect(contactPutBlock).not.toContain("res.json(result.rows[0]);");
  });

  it("routes allergy, medication, and condition acknowledgements through bounded helpers", () => {
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );
    const allergiesStart = subscriberRoute.indexOf("router.post('/profile/allergies'");
    const allergyDeleteStart = subscriberRoute.indexOf(
      "router.delete('/profile/allergies/:id'",
    );
    const medsStart = subscriberRoute.indexOf("router.post('/profile/medications'");
    const medDeleteStart = subscriberRoute.indexOf(
      "router.delete('/profile/medications/:id'",
    );
    const conditionsStart = subscriberRoute.indexOf("router.post('/profile/conditions'");
    const conditionDeleteStart = subscriberRoute.indexOf(
      "router.delete('/profile/conditions/:id'",
    );
    const contactPostStart = subscriberRoute.indexOf(
      "router.post('/profile/emergency-contacts'",
    );
    const contactDeleteStart = subscriberRoute.indexOf(
      "router.delete('/profile/emergency-contacts/:id'",
    );
    const alertSettingsStart = subscriberRoute.indexOf("router.get('/alert-settings'");

    const allergyPostBlock = subscriberRoute.slice(allergiesStart, allergyDeleteStart);
    const allergyDeleteBlock = subscriberRoute.slice(allergyDeleteStart, medsStart);
    const medicationPostBlock = subscriberRoute.slice(medsStart, medDeleteStart);
    const medicationDeleteBlock = subscriberRoute.slice(medDeleteStart, conditionsStart);
    const conditionPostBlock = subscriberRoute.slice(
      conditionsStart,
      conditionDeleteStart,
    );
    const conditionDeleteBlock = subscriberRoute.slice(
      conditionDeleteStart,
      contactPostStart,
    );
    const emergencyDeleteBlock = subscriberRoute.slice(
      contactDeleteStart,
      alertSettingsStart,
    );

    expect(subscriberRoute).toContain("buildPublicSubscriberAllergyWriteResponse");
    expect(subscriberRoute).toContain("buildPublicSubscriberMedicationWriteResponse");
    expect(subscriberRoute).toContain("buildPublicSubscriberConditionWriteResponse");
    expect(subscriberRoute).toContain("buildPublicSubscriberDeleteAcknowledgement");
    expect(allergyPostBlock).toContain(
      "res.status(201).json(buildPublicSubscriberAllergyWriteResponse({",
    );
    expect(medicationPostBlock).toContain(
      "res.status(201).json(buildPublicSubscriberMedicationWriteResponse({",
    );
    expect(conditionPostBlock).toContain(
      "res.status(201).json(buildPublicSubscriberConditionWriteResponse({",
    );
    expect(allergyPostBlock).not.toContain("res.status(201).json({ ...result.rows[0], odentity_claim });");
    expect(medicationPostBlock).not.toContain("res.status(201).json({ ...result.rows[0], odentity_claim });");
    expect(conditionPostBlock).not.toContain("res.status(201).json({ ...result.rows[0], odentity_claim });");
    expect(allergyDeleteBlock).toContain(
      "res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Allergy removed' }));",
    );
    expect(medicationDeleteBlock).toContain(
      "res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Medication removed' }));",
    );
    expect(conditionDeleteBlock).toContain(
      "res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Condition removed' }));",
    );
    expect(emergencyDeleteBlock).toContain(
      "res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Emergency contact removed' }));",
    );
    expect(allergyDeleteBlock).not.toContain("res.json({ message: 'Allergy removed', id: result.rows[0].id });");
    expect(medicationDeleteBlock).not.toContain("res.json({ message: 'Medication removed', id: result.rows[0].id });");
    expect(conditionDeleteBlock).not.toContain("res.json({ message: 'Condition removed', id: result.rows[0].id });");
    expect(emergencyDeleteBlock).not.toContain("res.json({ message: 'Emergency contact removed', id: result.rows[0].id });");
  });

  it("routes subscriber settings reads and writes through bounded helpers", () => {
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );
    const alertSettingsGetStart = subscriberRoute.indexOf(
      "router.get('/alert-settings'",
    );
    const alertSettingsPutStart = subscriberRoute.indexOf(
      "router.put('/alert-settings'",
    );
    const consentDefaultsGetStart = subscriberRoute.indexOf(
      "router.get('/consent-defaults'",
    );
    const consentDefaultsPutStart = subscriberRoute.indexOf(
      "router.put('/consent-defaults'",
    );
    const genericRoutesStart = subscriberRoute.indexOf(
      "// =============================================================================\n// GENERIC PARAM ROUTES",
    );

    const alertSettingsGetBlock = subscriberRoute.slice(
      alertSettingsGetStart,
      alertSettingsPutStart,
    );
    const alertSettingsPutBlock = subscriberRoute.slice(
      alertSettingsPutStart,
      consentDefaultsGetStart,
    );
    const consentDefaultsGetBlock = subscriberRoute.slice(
      consentDefaultsGetStart,
      consentDefaultsPutStart,
    );
    const consentDefaultsPutBlock = subscriberRoute.slice(
      consentDefaultsPutStart,
      genericRoutesStart,
    );

    expect(subscriberRoute).toContain("buildPublicAlertSettingsResponse");
    expect(subscriberRoute).toContain(
      "buildPublicAlertSettingsMutationResponse",
    );
    expect(subscriberRoute).toContain("buildPublicConsentDefaultsResponse");
    expect(subscriberRoute).toContain(
      "buildPublicConsentDefaultsMutationResponse",
    );
    expect(alertSettingsGetBlock).toContain(
      "res.json(buildPublicAlertSettingsResponse(result.rows[0]));",
    );
    expect(alertSettingsPutBlock).toContain(
      "res.json(buildPublicAlertSettingsMutationResponse({",
    );
    expect(consentDefaultsGetBlock).toContain(
      "res.json(buildPublicConsentDefaultsResponse(result.rows[0]));",
    );
    expect(consentDefaultsPutBlock).toContain(
      "res.json(buildPublicConsentDefaultsMutationResponse({",
    );
    expect(alertSettingsGetBlock).not.toContain("res.json({\n      alert_sensitivity:");
    expect(alertSettingsPutBlock).not.toContain("res.json({\n      alert_sensitivity:");
    expect(consentDefaultsGetBlock).not.toContain("res.json({\n      default_scope:");
    expect(consentDefaultsPutBlock).not.toContain("res.json({\n      default_scope:");
  });
});
