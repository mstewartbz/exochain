function buildPublicOdentityScoreDimensionResponse(dimension = {}) {
  return {
    dimension: dimension.dimension,
    label: dimension.label,
    weight: dimension.weight == null ? null : Number(dimension.weight),
    current_score:
      dimension.current_score == null ? null : parseFloat(dimension.current_score),
    max_possible:
      dimension.max_possible == null ? null : parseFloat(dimension.max_possible),
    claim_count:
      dimension.claim_count == null ? null : Number(dimension.claim_count),
  };
}

function buildPublicOdentityScoreResponse({
  dimensions = [],
  compositeScore = null,
  polygonAreaPercentage = null,
} = {}) {
  return {
    dimensions: dimensions.map(buildPublicOdentityScoreDimensionResponse),
    composite_score: compositeScore == null ? null : Number(compositeScore),
    polygon_area_percentage:
      polygonAreaPercentage == null ? null : Number(polygonAreaPercentage),
  };
}

module.exports = {
  buildPublicOdentityScoreDimensionResponse,
  buildPublicOdentityScoreResponse,
};
