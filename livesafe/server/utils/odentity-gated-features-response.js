function buildPublicOdentityGatedFeatureResponse(gatedFeature = {}) {
  return {
    score_minimum:
      gatedFeature.score_minimum == null ? null : gatedFeature.score_minimum,
    feature: gatedFeature.feature || null,
    label: gatedFeature.label || null,
    unlocked: Boolean(gatedFeature.unlocked),
  };
}

function buildPublicOdentityGatedFeaturesResponse({
  compositeScore,
  gatedFeatures = [],
} = {}) {
  return {
    composite_score:
      compositeScore == null ? null : Math.round(compositeScore * 100) / 100,
    gated_features: gatedFeatures.map(buildPublicOdentityGatedFeatureResponse),
  };
}

module.exports = {
  buildPublicOdentityGatedFeatureResponse,
  buildPublicOdentityGatedFeaturesResponse,
};
