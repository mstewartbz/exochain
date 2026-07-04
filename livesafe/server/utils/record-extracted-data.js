"use strict";

function createRecordParseFailureMetadata({
  format,
  stage,
  code = "structured_data_parse_failed",
  parsedAt = new Date().toISOString(),
} = {}) {
  return {
    format: format || "unknown",
    parsed_at: parsedAt,
    parse_status: "failed",
    parse_error: code,
    parse_error_stage: stage || "unknown",
  };
}

module.exports = {
  createRecordParseFailureMetadata,
};
