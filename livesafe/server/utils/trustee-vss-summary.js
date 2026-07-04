function hasVssShardReference(shardRef) {
  return (
    typeof shardRef === "string"
    && (
      shardRef.startsWith("vss:exo:shard:")
      || shardRef.startsWith("shard:exo:")
    )
  );
}

function buildTrusteeVssStatusSummary(trustee = {}) {
  const hasVssShard = hasVssShardReference(trustee.shard_ref);

  return {
    has_vss_shard: hasVssShard,
    shard_status: hasVssShard ? "present" : "missing",
  };
}

module.exports = {
  buildTrusteeVssStatusSummary,
  hasVssShardReference,
};
