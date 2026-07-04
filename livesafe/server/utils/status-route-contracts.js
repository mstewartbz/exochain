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

    app.get(contract.path, (req, res) => {
      applyStatusRouteHeaders(res);
      responder(req, res);
    });
    app.all(contract.path, methodNotAllowed);
  }
}

module.exports = {
  getStatusRouteContracts,
  registerStatusRouteContracts,
};
