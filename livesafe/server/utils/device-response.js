"use strict";

function buildDeviceSummaryResponse(row) {
  return {
    device_id: row.device_id,
    device_name: row.device_name,
    is_active: Boolean(row.is_active),
    revoked_at: row.revoked_at || null,
    last_used_at: row.last_used_at || null,
    created_at: row.created_at || null,
  };
}

function buildDeviceRegistrationResponse(row, token) {
  return {
    device_id: row.device_id,
    device_name: row.device_name,
    is_active: Boolean(row.is_active),
    created_at: row.created_at || null,
    token,
  };
}

function buildDeviceListResponse(rows) {
  const devices = rows.map(buildDeviceSummaryResponse);

  return {
    devices,
    total: devices.length,
    active: devices.filter((device) => device.is_active).length,
  };
}

function buildDeviceRevocationResponse(row) {
  return {
    message: "Device key revoked successfully",
    device_id: row.device_id,
    device_name: row.device_name,
    revoked_at: row.revoked_at || null,
  };
}

function buildDeviceVerificationResponse(row) {
  if (!row.is_active) {
    return {
      valid: false,
      error: "Device has been revoked",
      device_bound: true,
      device_id: row.device_id,
      device_name: row.device_name,
      revoked_at: row.revoked_at || null,
    };
  }

  return {
    valid: true,
    device_bound: true,
    device_id: row.device_id,
    device_name: row.device_name,
  };
}

module.exports = {
  buildDeviceListResponse,
  buildDeviceRegistrationResponse,
  buildDeviceRevocationResponse,
  buildDeviceSummaryResponse,
  buildDeviceVerificationResponse,
};
