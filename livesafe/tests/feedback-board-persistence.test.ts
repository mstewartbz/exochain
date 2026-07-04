import {
  buildFeedbackBoardPersistenceNamespace,
  feedbackBoardPersistenceSurfaces,
  isSupportedFeedbackBoardPersistenceSurface,
  type FeedbackBoardPersistenceSurface
} from "../src/feedback-board-persistence.js";

describe("feedback board persistence namespace", () => {
  it("builds the required livesafe key inventory for supported surfaces", () => {
    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-item", {
        feedbackId: "feedback:trust_banner_bug"
      })
    ).toEqual({
      surface: "feedback-item",
      key: "livesafe:feedback:item:feedback:trust_banner_bug"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-board", {
        status: "planning"
      })
    ).toEqual({
      surface: "feedback-board",
      key: "livesafe:feedback:board:planning"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-by-target", {
        targetType: "trust-state",
        targetId: "trust-state:banner"
      })
    ).toEqual({
      surface: "feedback-by-target",
      key: "livesafe:feedback:by_target:trust-state:trust-state:banner"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-by-work-batch", {
        workBatchTag: "batch:trust-copy"
      })
    ).toEqual({
      surface: "feedback-by-work-batch",
      key: "livesafe:feedback:by_work_batch:batch:trust-copy"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-index-all")
    ).toEqual({
      surface: "feedback-index-all",
      key: "livesafe:feedback:index:all"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-activities", {
        feedbackId: "feedback:trust_banner_bug"
      })
    ).toEqual({
      surface: "feedback-activities",
      key: "livesafe:feedback:activities:feedback:trust_banner_bug"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-votes", {
        feedbackId: "feedback:trust_banner_bug"
      })
    ).toEqual({
      surface: "feedback-votes",
      key: "livesafe:feedback:votes:feedback:trust_banner_bug"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-stats-by-category")
    ).toEqual({
      surface: "feedback-stats-by-category",
      key: "livesafe:feedback:stats:by_category"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-stats-by-target-type")
    ).toEqual({
      surface: "feedback-stats-by-target-type",
      key: "livesafe:feedback:stats:by_target_type"
    });

    expect(
      buildFeedbackBoardPersistenceNamespace("feedback-stats-by-status")
    ).toEqual({
      surface: "feedback-stats-by-status",
      key: "livesafe:feedback:stats:by_status"
    });
  });

  it("fails closed for malformed ids or unsupported parameter shapes", () => {
    expect(() =>
      buildFeedbackBoardPersistenceNamespace("feedback-item", {
        feedbackId: "feedback trust banner"
      })
    ).toThrow(
      "feedback-item feedbackId must match /^[A-Za-z0-9:_-]+$/."
    );

    expect(() =>
      buildFeedbackBoardPersistenceNamespace("feedback-board", {
        status: "queued"
      })
    ).toThrow("feedback-board status must be one of new, backlog, planning, development, testing, validation, deployed, held.");

    expect(() =>
      buildFeedbackBoardPersistenceNamespace("feedback-by-target", {
        targetType: "trust-state"
      })
    ).toThrow("feedback-by-target requires targetId.");

    expect(() =>
      buildFeedbackBoardPersistenceNamespace("feedback-index-all", {
        feedbackId: "feedback:unexpected"
      })
    ).toThrow("feedback-index-all does not accept parameters.");
  });

  it("exposes the bounded feedback-board persistence surface vocabulary", () => {
    const supported: FeedbackBoardPersistenceSurface[] = [
      "feedback-item",
      "feedback-board",
      "feedback-by-target",
      "feedback-by-work-batch",
      "feedback-index-all",
      "feedback-activities",
      "feedback-votes",
      "feedback-stats-by-category",
      "feedback-stats-by-target-type",
      "feedback-stats-by-status"
    ];

    expect(feedbackBoardPersistenceSurfaces).toEqual(supported);
    expect(supported.filter(isSupportedFeedbackBoardPersistenceSurface)).toEqual(
      supported
    );
    expect(isSupportedFeedbackBoardPersistenceSurface("help-session")).toBe(
      false
    );
  });
});
