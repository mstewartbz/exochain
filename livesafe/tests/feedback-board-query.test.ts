import { describe, expect, it } from "vitest";

import {
  buildFeedbackBoardReadContract,
  feedbackBoardReadOperations,
  isFeedbackBoardReadOperation,
  type FeedbackBoardReadOperation
} from "../src/feedback-board-query.js";

describe("feedback board typed-query contract", () => {
  it("normalizes supported board and aggregate query filters while staying read-only", () => {
    const boardContract = buildFeedbackBoardReadContract({
      operation: "query-feedback-board",
      statuses: ["planning", "deployed", "planning"],
      targetType: "trust-state",
      category: "documentation-gap",
      workBatchTag: "batch:trust-copy"
    });

    expect(boardContract).toEqual({
      operation: "query-feedback-board",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "Feedback-board query routes remain disabled until a backend is selected and tested."
      ],
      filters: {
        statuses: ["planning", "deployed"],
        targetType: "trust-state",
        category: "documentation-gap",
        workBatchTag: "batch:trust-copy"
      }
    });

    const statsContract = buildFeedbackBoardReadContract({
      operation: "query-feedback-stats",
      statuses: ["held", "new", "held"]
    });

    expect(statsContract.filters.statuses).toEqual(["new", "held"]);
  });

  it("requires the bounded ids needed for target, work-batch, item, and activity queries", () => {
    expect(
      buildFeedbackBoardReadContract({
        operation: "query-feedback-by-target",
        targetType: "trust-state",
        targetId: "trust-state:banner"
      })
    ).toEqual({
      operation: "query-feedback-by-target",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "Feedback-board query routes remain disabled until a backend is selected and tested."
      ],
      filters: {
        statuses: [],
        targetType: "trust-state",
        targetId: "trust-state:banner"
      }
    });

    expect(
      buildFeedbackBoardReadContract({
        operation: "query-feedback-by-work-batch",
        workBatchTag: "batch:trust-copy"
      })
    ).toEqual({
      operation: "query-feedback-by-work-batch",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "Feedback-board query routes remain disabled until a backend is selected and tested."
      ],
      filters: {
        statuses: [],
        workBatchTag: "batch:trust-copy"
      }
    });

    expect(
      buildFeedbackBoardReadContract({
        operation: "query-feedback-item",
        feedbackId: "feedback:target-match"
      })
    ).toEqual({
      operation: "query-feedback-item",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "Feedback-board query routes remain disabled until a backend is selected and tested."
      ],
      filters: {
        statuses: [],
        feedbackId: "feedback:target-match"
      }
    });

    expect(
      buildFeedbackBoardReadContract({
        operation: "query-feedback-activity-log",
        feedbackId: "feedback:target-match"
      })
    ).toEqual({
      operation: "query-feedback-activity-log",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "Feedback-board query routes remain disabled until a backend is selected and tested."
      ],
      filters: {
        statuses: [],
        feedbackId: "feedback:target-match"
      }
    });
  });

  it("fails closed for unsupported operations, malformed ids, or missing required filters", () => {
    expect(() =>
      buildFeedbackBoardReadContract({
        operation: "delete-feedback"
      })
    ).toThrow("Unsupported feedback-board read operation: delete-feedback.");

    expect(() =>
      buildFeedbackBoardReadContract({
        operation: "query-feedback-board",
        statuses: ["queued"]
      })
    ).toThrow("Unsupported feedback status filter: queued.");

    expect(() =>
      buildFeedbackBoardReadContract({
        operation: "query-feedback-by-target",
        targetType: "trust-state"
      })
    ).toThrow("query-feedback-by-target requires targetId.");

    expect(() =>
      buildFeedbackBoardReadContract({
        operation: "query-feedback-by-work-batch",
        workBatchTag: "batch/trust-copy"
      })
    ).toThrow(
      "query-feedback-by-work-batch workBatchTag must match /^[A-Za-z0-9:_-]+$/."
    );

    expect(() =>
      buildFeedbackBoardReadContract({
        operation: "query-feedback-item",
        feedbackId: "feedback target"
      })
    ).toThrow("query-feedback-item feedbackId must match /^[A-Za-z0-9:_-]+$/.");
  });

  it("exposes the bounded feedback-board read vocabulary", () => {
    const supported: FeedbackBoardReadOperation[] = [
      "query-feedback-board",
      "query-feedback-by-target",
      "query-feedback-by-work-batch",
      "query-feedback-item",
      "query-feedback-activity-log",
      "query-feedback-counts-by-target",
      "query-feedback-stats"
    ];

    expect(feedbackBoardReadOperations).toEqual(supported);
    expect(supported.filter(isFeedbackBoardReadOperation)).toEqual(supported);
    expect(isFeedbackBoardReadOperation("create-feedback")).toBe(false);
  });
});
