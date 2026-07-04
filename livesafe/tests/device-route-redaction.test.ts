const fs = require("node:fs");
const path = require("node:path");

describe("device route redaction wiring", () => {
  it("routes device responses through bounded helpers and revokes by device_id", () => {
    const deviceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/devices.js"),
      "utf8",
    );

    expect(deviceRoute).toContain("buildDeviceRegistrationResponse");
    expect(deviceRoute).toContain("buildDeviceListResponse");
    expect(deviceRoute).toContain("buildDeviceRevocationResponse");
    expect(deviceRoute).toContain("buildDeviceVerificationResponse");
    expect(deviceRoute).toContain("router.delete('/:deviceId'");
    expect(deviceRoute).toContain("WHERE device_id = $1 AND subscriber_id = $2");
    expect(deviceRoute).not.toContain("router.delete('/:keyRef'");
    expect(deviceRoute).not.toContain("revoked_reason: deviceKey.revoked_reason");
    expect(deviceRoute).not.toContain("key_ref = $1 AND subscriber_id = $2");
    expect(deviceRoute).not.toContain("SELECT id, device_id, key_ref, device_name, is_active, revoked_at, revoked_reason");
    expect(deviceRoute).not.toContain("res.status(201).json({");
    expect(deviceRoute).not.toContain("res.json({\n      message: 'Device key revoked successfully'");
  });
});
