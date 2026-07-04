/**
 * Route error handling utilities.
 */

function isMulterError(err) {
  return Boolean(
    err &&
      (err.code === "LIMIT_FILE_SIZE" ||
        err.code === "LIMIT_UNEXPECTED_FILE" ||
        (typeof err.message === "string" &&
          err.message.includes("File type not allowed"))),
  );
}

function isDatabaseError(err) {
  return Boolean(
    err &&
      (err.isDbError ||
        err.status === 503 ||
        err.code === "DB_UNAVAILABLE" ||
        err.code === "ECONNREFUSED" ||
        err.code === "57P01" ||
        err.code === "08006" ||
        err.code === "08001" ||
        err.code === "08004" ||
        err.code === "3D000" ||
        err.code === "53300" ||
        (typeof err.message === "string" &&
          (err.message.includes("Connection refused") ||
            err.message.includes("ECONNREFUSED") ||
            err.message.includes("connection refused") ||
            err.message.includes("the database system is") ||
            err.message.includes("Connection terminated") ||
            err.message.includes("cannot connect to server") ||
            err.message.includes("pool is draining")))),
  );
}

function createErrorResponse({ err, fallbackMessage, correlationId } = {}) {
  if (isMulterError(err)) {
    return {
      status: 400,
      body: {
        error: "File upload rejected.",
        code: "INVALID_FILE_UPLOAD",
        ...(correlationId ? { correlationId } : {}),
      },
    };
  }

  if (isDatabaseError(err)) {
    return {
      status: 503,
      body: {
        error: "The database is temporarily unavailable. Please try again in a moment.",
        code: "DB_UNAVAILABLE",
        ...(correlationId ? { correlationId } : {}),
      },
    };
  }

  const status =
    Number.isInteger(err?.status) && err.status >= 400 && err.status <= 599
      ? err.status
      : 500;
  const isClientError = status >= 400 && status < 500;

  return {
    status,
    body: {
      error:
        fallbackMessage ||
        (isClientError
          ? "Request could not be processed."
          : "Internal server error"),
      code: isClientError ? "REQUEST_REJECTED" : "INTERNAL_ERROR",
      ...(correlationId ? { correlationId } : {}),
    },
  };
}

function sendError(res, err, fallbackMessage, options = {}) {
  const { status, body } = createErrorResponse({
    err,
    fallbackMessage,
    correlationId: options.correlationId,
  });
  return res.status(status).json(body);
}

module.exports = {
  createErrorResponse,
  isDatabaseError,
  isMulterError,
  sendError,
};
