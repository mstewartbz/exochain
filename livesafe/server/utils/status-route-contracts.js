function applyStatusRouteHeaders(res) {
  res.setHeader("Cache-Control", "no-store");
}

function buildMethodNotAllowedHandler() {
  return (_req, res) => {
    applyStatusRouteHeaders(res);
    res.setHeader("Allow", "GET");
    res.status(405).json({ error: "Method Not Allowed", allowed_methods: ["GET"] });
  };
}

function buildStatusRouteResponderError() {
  const error = new Error("Status route responder failed.");
  error.code = "STATUS_ROUTE_RESPONDER_FAILED";
  error.status = 500;
  return error;
}

function getStatusRouteContracts() {
  return [
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
  ];
}

function registerStatusRouteContracts(app, responders) {
  const methodNotAllowed = buildMethodNotAllowedHandler();

  for (const contract of getStatusRouteContracts()) {
    const responder = responders[contract.responder];

    if (typeof responder !== "function") {
      throw new Error(`Missing status route responder: ${contract.responder}`);
    }

    app.get(contract.path, async (req, res, next) => {
      applyStatusRouteHeaders(res);
      try {
        await responder(req, res, next);
      } catch {
        const error = buildStatusRouteResponderError();

        if (typeof next === "function") {
          next(error);
          return;
        }

        throw error;
      }
    });
    app.all(contract.path, methodNotAllowed);
  }
}

module.exports = {
  buildStatusRouteResponderError,
  getStatusRouteContracts,
  registerStatusRouteContracts,
};
