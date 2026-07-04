"use strict";

function createHealthStatusOkPayload({
  databaseTimestamp,
  exochainConnected,
  version,
  uptime,
}) {
  return {
    status: "ok",
    database: "connected",
    exochain_connected: Boolean(exochainConnected),
    timestamp: databaseTimestamp,
    version,
    uptime,
  };
}

function createHealthStatusErrorPayload({
  exochainConnected,
}) {
  return {
    status: "error",
    database: "disconnected",
    exochain_connected: Boolean(exochainConnected),
    error: "Database temporarily unavailable.",
    code: "DATABASE_UNAVAILABLE",
    retryable: true,
  };
}

async function sendHealthStatusResponse(_req, res, { pool, exochainConnected, version, uptime }) {
  try {
    const dbResult = await pool.query("SELECT NOW() as time");
    return res.json(
      createHealthStatusOkPayload({
        databaseTimestamp: dbResult.rows[0].time,
        exochainConnected,
        version,
        uptime,
      }),
    );
  } catch {
    return res
      .status(503)
      .json(createHealthStatusErrorPayload({ exochainConnected }));
  }
}

module.exports = {
  createHealthStatusErrorPayload,
  createHealthStatusOkPayload,
  sendHealthStatusResponse,
};
