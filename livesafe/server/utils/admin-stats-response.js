"use strict";

function toCount(value) {
  return Number.parseInt(value ?? 0, 10) || 0;
}

function buildAdminStatsResponse({
  subscribers = {},
  providers = 0,
  medical_records = 0,
  scans = 0,
} = {}) {
  return {
    subscribers: {
      total: toCount(subscribers.total),
      admins: toCount(subscribers.admins),
    },
    providers: toCount(providers),
    medical_records: toCount(medical_records),
    scans: toCount(scans),
  };
}

module.exports = {
  buildAdminStatsResponse,
};
