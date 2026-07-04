const fs = require("node:fs");
const path = require("node:path");

describe("consent route redaction wiring", () => {
  it("routes direct consent-event responses through bounded helpers instead of raw rows", () => {
    const consentRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/consent.js"),
      "utf8",
    );

    expect(consentRoute).toContain("buildConsentResponse");
    expect(consentRoute).toContain("buildConsentListResponse");
    expect(consentRoute).toContain("buildConsentCollectionResponse");
    expect(consentRoute).toContain("buildConsentGrantAcknowledgement");
    expect(consentRoute).toContain("buildConsentRevocationAcknowledgement");
    expect(consentRoute).toContain("buildConsentAccessCheckResponse");
    expect(consentRoute).toContain("buildConsentExpiryCheckResponse");
    expect(consentRoute).toContain("buildConsentProviderListResponse");
    expect(consentRoute).toContain("buildSubscriberAccessRequestListResponse");
    expect(consentRoute).toContain("buildSubscriberAccessRequestApprovalResponse");
    expect(consentRoute).toContain("buildSubscriberAccessRequestDenialResponse");
    expect(consentRoute).toContain("buildProviderAccessRequestResponse");
    expect(consentRoute).toContain("buildProviderAccessRequestCreateAcknowledgement");
    expect(consentRoute).toContain("buildProviderAccessRequestListResponse");
    expect(consentRoute).toContain("router.get('/my-consents'");
    expect(consentRoute).toContain("router.get('/providers'");
    expect(consentRoute).toContain("router.get('/access-requests'");
    expect(consentRoute).toContain("res.json(buildConsentListResponse(result.rows));");
    expect(consentRoute).toContain("res.json(buildConsentProviderListResponse(result.rows));");
    expect(consentRoute).toContain("res.json(buildSubscriberAccessRequestListResponse(result.rows));");
    expect(consentRoute).toContain("res.json(buildProviderAccessRequestListResponse(requests));");
    expect(consentRoute).toContain("res.json(buildConsentCollectionResponse(result.rows));");
    const requestAccessBlock = consentRoute.slice(
      consentRoute.indexOf("router.post('/request-access'"),
      consentRoute.indexOf("router.post('/access-requests/:id/approve'"),
    );
    expect(consentRoute).not.toContain("res.json({ has_access: true, consent: result.rows[0] });");
    expect(consentRoute).not.toContain("res.json(result.rows);");
    expect(requestAccessBlock).not.toContain("res.status(201).json({");
    expect(consentRoute).not.toContain("request: { ...accessRequest, status: 'approved', consent_id: consent.id },");
    expect(consentRoute).not.toContain("res.json({ ...result.rows[0], message: buildConsentRevocationSuccessMessage() });");
    expect(consentRoute).not.toContain("return res.status(200).json({\n        consent: existing,");
    expect(consentRoute).not.toContain("consent: {\n        ...consent,");
    expect(consentRoute).not.toContain("return res.json({ has_access: false });");
    expect(consentRoute).not.toContain("res.json({ has_access: true, consent: buildConsentResponse(result.rows[0]) });");
    expect(consentRoute).not.toContain("consent_ids: notifiedConsents");
    expect(consentRoute).not.toContain("res.json({ message: 'Access request denied' });");
    expect(consentRoute).toContain("return res.status(200).json(");
    expect(consentRoute).toContain("buildConsentGrantAcknowledgement({");
    expect(consentRoute).toContain("buildConsentRevocationAcknowledgement({");
    expect(consentRoute).toContain("buildConsentAccessCheckResponse(");
    expect(consentRoute).toContain("buildConsentExpiryCheckResponse({");
    expect(consentRoute).toContain("buildProviderAccessRequestCreateAcknowledgement({");
    expect(consentRoute).toContain("subscriber_name: [subscriber.first_name, subscriber.last_name]");
    expect(consentRoute).toContain("buildSubscriberAccessRequestApprovalResponse({");
    expect(consentRoute).toContain("buildSubscriberAccessRequestDenialResponse()");
  });
});
