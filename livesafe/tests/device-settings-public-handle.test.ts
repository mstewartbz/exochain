const fs = require("node:fs");
const path = require("node:path");

describe("device settings public handle", () => {
  it("uses device_id rather than key_ref in the subscriber settings UI", () => {
    const settingsPage = fs.readFileSync(
      path.join(process.cwd(), "client/src/pages/Settings.jsx"),
      "utf8",
    );

    expect(settingsPage).toContain("handleRevokeDevice(device.device_id, device.device_name)");
    expect(settingsPage).toContain("disabled={revokingDevice === device.device_id}");
    expect(settingsPage).toContain("{revokingDevice === device.device_id ? 'Revoking…' : 'Revoke'}");
    expect(settingsPage).toContain("key={device.device_id}");
    expect(settingsPage).not.toContain("handleRevokeDevice(device.key_ref, device.device_name)");
    expect(settingsPage).not.toContain("disabled={revokingDevice === device.key_ref}");
    expect(settingsPage).not.toContain("key={device.key_ref}");
  });
});
