import { describe, expect, it } from "vitest";

const {
  getStatusRouteContracts,
  registerStatusRouteContracts,
} = require("../server/utils/status-route-contracts.js");

describe("status route contracts", () => {
  it("defines the documented read-only status routes with GET-only guards", () => {
    expect(getStatusRouteContracts()).toEqual([
      {
        path: "/api/health",
        responder: "sendHealthResponse",
      },
      {
        path: "/api/trust/status",
        responder: "sendTrustStatusResponse",
      },
      {
        path: "/api/help/status",
        responder: "sendAiHelpStatusResponse",
      },
      {
        path: "/api/help/usage-summary/status",
        responder: "sendAiHelpUsageSummaryStatusResponse",
      },
      {
        path: "/api/help/session-transcript/status",
        responder: "sendAiHelpSessionTranscriptStatusResponse",
      },
      {
        path: "/api/help/unanswered-topics/status",
        responder: "sendAiHelpUnansweredTopicStatusResponse",
      },
      {
        path: "/api/help/feedback-board/status",
        responder: "sendFeedbackBoardStatusResponse",
      },
      {
        path: "/api/help/feedback-code-hints/status",
        responder: "sendFeedbackCodeHintsStatusResponse",
      },
    ]);
  });

  it("registers GET handlers and method guards for every status route", () => {
    const registrations: Array<{
      method: "GET" | "ALL";
      path: string;
      handler: unknown;
    }> = [];
    const app = {
      get(path: string, handler: unknown) {
        registrations.push({ method: "GET", path, handler });
      },
      all(path: string, handler: unknown) {
        registrations.push({ method: "ALL", path, handler });
      },
    };

    registerStatusRouteContracts(app, {
      sendHealthResponse() {},
      sendTrustStatusResponse() {},
      sendAiHelpStatusResponse() {},
      sendAiHelpUsageSummaryStatusResponse() {},
      sendAiHelpSessionTranscriptStatusResponse() {},
      sendAiHelpUnansweredTopicStatusResponse() {},
      sendFeedbackBoardStatusResponse() {},
      sendFeedbackCodeHintsStatusResponse() {},
    });

    expect(registrations).toHaveLength(16);
    expect(registrations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          method: "GET",
          path: "/api/health",
        }),
        expect.objectContaining({
          method: "ALL",
          path: "/api/health",
        }),
        expect.objectContaining({
          method: "GET",
          path: "/api/help/unanswered-topics/status",
        }),
        expect.objectContaining({
          method: "ALL",
          path: "/api/help/unanswered-topics/status",
        }),
        expect.objectContaining({
          method: "GET",
          path: "/api/trust/status",
        }),
        expect.objectContaining({
          method: "ALL",
          path: "/api/trust/status",
        }),
        expect.objectContaining({
          method: "GET",
          path: "/api/help/feedback-code-hints/status",
        }),
        expect.objectContaining({
          method: "ALL",
          path: "/api/help/feedback-code-hints/status",
        }),
      ]),
    );
  });

  it("applies no-store cache headers to status responders and method guards", () => {
    const handlers = new Map<string, { get?: Function; all?: Function }>();
    const app = {
      get(path: string, handler: Function) {
        handlers.set(path, { ...handlers.get(path), get: handler });
      },
      all(path: string, handler: Function) {
        handlers.set(path, { ...handlers.get(path), all: handler });
      },
    };

    registerStatusRouteContracts(app, {
      sendHealthResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
      sendTrustStatusResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
      sendAiHelpStatusResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
      sendAiHelpUsageSummaryStatusResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
      sendAiHelpSessionTranscriptStatusResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
      sendAiHelpUnansweredTopicStatusResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
      sendFeedbackBoardStatusResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
      sendFeedbackCodeHintsStatusResponse(_req: unknown, res: any) {
        res.status(200).json({ ok: true });
      },
    });

    const responseHeaders = new Map<string, string>();
    const response = {
      statusCode: 200,
      setHeader(name: string, value: string) {
        responseHeaders.set(name.toLowerCase(), value);
      },
      status(code: number) {
        this.statusCode = code;
        return this;
      },
      json(payload: unknown) {
        return payload;
      },
    };

    handlers.get("/api/health")?.get?.({}, response);
    expect(responseHeaders.get("cache-control")).toBe("no-store");

    responseHeaders.clear();
    handlers.get("/api/health")?.all?.({}, response);
    expect(response.statusCode).toBe(405);
    expect(responseHeaders.get("allow")).toBe("GET");
    expect(responseHeaders.get("cache-control")).toBe("no-store");
  });
});
