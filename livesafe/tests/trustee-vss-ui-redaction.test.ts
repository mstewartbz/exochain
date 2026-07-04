import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe('trustee VSS UI redaction', () => {
  it('does not render raw shard references on the trustee dashboard', () => {
    const dashboard = fs.readFileSync(
      path.join(process.cwd(), 'client/src/pages/TrusteeDashboard.jsx'),
      'utf8',
    );

    expect(dashboard).not.toContain('{t.shard_ref}');
    expect(dashboard).toContain('t.has_vss_shard');
    expect(dashboard).toContain('t.shard_status');
  });

  it('does not render raw shard references on the trustee subscriber detail page', () => {
    const detailPage = fs.readFileSync(
      path.join(process.cwd(), 'client/src/pages/TrusteeSubscriberDetail.jsx'),
      'utf8',
    );

    expect(detailPage).not.toContain('{detail.my_trusteeship.shard_ref}');
    expect(detailPage).toContain('detail.my_trusteeship.has_vss_shard');
    expect(detailPage).toContain('detail.my_trusteeship.shard_status');
  });
});
