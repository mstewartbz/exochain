import { describe, expect, it, vi } from "vitest";

const {
  createErrorResponse,
  sendError,
} = require("../server/utils/errorHandler.js");

describe("error handler redaction", () => {
  it("redacts raw upload failure details from public error payloads", () => {
    const payload = createErrorResponse({
      err: {
        code: "LIMIT_UNEXPECTED_FILE",
        message: "avatar.exe rejected because application/x-msdownload is not allowed",
      },
    });

    expect(payload).toEqual({
      status: 400,
      body: {
        error: "File upload rejected.",
        code: "INVALID_FILE_UPLOAD",
      },
    });
  });

  it("redacts raw database failure details from public error payloads", () => {
    const payload = createErrorResponse({
      err: {
        code: "08006",
        message: "password authentication failed for user postgres",
      },
    });

    expect(payload).toEqual({
      status: 503,
      body: {
        error: "The database is temporarily unavailable. Please try again in a moment.",
        code: "DB_UNAVAILABLE",
      },
    });
  });

  it("redacts unexpected exception details from public error payloads", () => {
    const payload = createErrorResponse({
      err: {
        status: 500,
        message: "Cannot read properties of undefined (reading 'secretKey')",
      },
      fallbackMessage: "Registration failed. Please try again.",
    });

    expect(payload).toEqual({
      status: 500,
      body: {
        error: "Registration failed. Please try again.",
        code: "INTERNAL_ERROR",
      },
    });
  });

  it("sends the redacted payload through the shared responder", () => {
    const status = vi.fn(function (
      this: { statusCode: number; payload: unknown },
      code: number,
    ) {
      this.statusCode = code;
      return this;
    });
    const json = vi.fn(function (
      this: { statusCode: number; payload: unknown },
      payload: unknown,
    ) {
      this.payload = payload;
      return payload;
    });
    const res = { statusCode: 200, payload: null, status, json };

    sendError(
      res,
      {
        message: "JWT secret leaked in stack trace",
      },
      "Login failed. Please try again.",
    );

    expect(status).toHaveBeenCalledWith(500);
    expect(json).toHaveBeenCalledWith({
      error: "Login failed. Please try again.",
      code: "INTERNAL_ERROR",
    });
  });
});
