const {
  buildDeviceRegistrationResponse,
  buildDeviceListResponse,
  buildDeviceRevocationResponse,
  buildDeviceVerificationResponse,
} = require("../server/utils/device-response.js");

describe("device response redaction", () => {
  const deviceRow = {
    id: 9,
    subscriber_id: 88,
    device_id: "device-ios-123",
    key_ref: "dkey:private-secret-reference",
    public_key: "-----BEGIN PUBLIC KEY-----secret-----END PUBLIC KEY-----",
    device_name: "Bob's iPhone",
    is_active: false,
    revoked_at: "2026-06-06T07:42:00.000Z",
    revoked_reason: "User revoked",
    revoked_by: 88,
    last_used_at: "2026-06-06T07:30:00.000Z",
    created_at: "2026-06-06T07:00:00.000Z",
  };

  it("builds a bounded device registration payload without key references", () => {
    expect(
      buildDeviceRegistrationResponse(deviceRow, "device-token-value"),
    ).toEqual({
      device_id: "device-ios-123",
      device_name: "Bob's iPhone",
      is_active: false,
      created_at: "2026-06-06T07:00:00.000Z",
      token: "device-token-value",
    });
  });

  it("builds a bounded device list response without key refs or revoke reasons", () => {
    expect(buildDeviceListResponse([deviceRow])).toEqual({
      devices: [
        {
          device_id: "device-ios-123",
          device_name: "Bob's iPhone",
          is_active: false,
          revoked_at: "2026-06-06T07:42:00.000Z",
          last_used_at: "2026-06-06T07:30:00.000Z",
          created_at: "2026-06-06T07:00:00.000Z",
        },
      ],
      total: 1,
      active: 0,
    });
  });

  it("builds a bounded device revocation payload without key refs", () => {
    expect(buildDeviceRevocationResponse(deviceRow)).toEqual({
      message: "Device key revoked successfully",
      device_id: "device-ios-123",
      device_name: "Bob's iPhone",
      revoked_at: "2026-06-06T07:42:00.000Z",
    });
  });

  it("builds a bounded device verification payload without key refs or revoke reasons", () => {
    expect(buildDeviceVerificationResponse(deviceRow)).toEqual({
      valid: false,
      error: "Device has been revoked",
      device_bound: true,
      device_id: "device-ios-123",
      device_name: "Bob's iPhone",
      revoked_at: "2026-06-06T07:42:00.000Z",
    });
  });
});
