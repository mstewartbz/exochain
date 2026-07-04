import { describe, expect, it } from "vitest";

const {
  buildTrusteeVssStatusSummary,
} = require('../server/utils/trustee-vss-summary.js');

describe('trustee VSS summary redaction', () => {
  it('returns bounded shard status without exposing the raw shard reference', () => {
    const summary = buildTrusteeVssStatusSummary({
      shard_ref: 'vss:exo:shard:secret-primary',
    });

    expect(summary).toEqual({
      has_vss_shard: true,
      shard_status: 'present',
    });
    expect(summary).not.toHaveProperty('shard_ref');
    expect(JSON.stringify(summary)).not.toContain('secret-primary');
  });

  it('marks missing shard state without exposing fallback values', () => {
    const summary = buildTrusteeVssStatusSummary({
      shard_ref: null,
    });

    expect(summary).toEqual({
      has_vss_shard: false,
      shard_status: 'missing',
    });
  });
});
