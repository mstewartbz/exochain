import React from 'react';

const DIMENSION_COLORS = {
  identity_core: '#3B82F6',
  health_record_completeness: '#10B981',
  pace_trust_network: '#F59E0B',
  provider_trust: '#8B5CF6',
  responder_accessibility: '#EF4444',
  credential_issuers: '#EC4899',
};

const DIMENSION_LABELS = [
  'Core Identity',
  'Medical Record Completeness',
  'PACE Trust Network',
  'Provider Trust',
  'First Responder Accessibility',
  'External Credential Issuers',
];

function OdentityPolarGraph({ dimensions = [], compositeScore = 0, polygonAreaPercentage = 0, onDimensionClick = null }) {
  const cx = 200;
  const cy = 200;
  const maxRadius = 150;
  const numAxes = 6;
  const angleStep = (2 * Math.PI) / numAxes;
  // Start from top (north) = -PI/2
  const startAngle = -Math.PI / 2;

  // Use provided dimensions or defaults
  const dimData = dimensions.length === numAxes
    ? dimensions
    : DIMENSION_LABELS.map((label, i) => ({
        dimension: ['identity_core', 'health_record_completeness', 'pace_trust_network', 'provider_trust', 'responder_accessibility', 'credential_issuers'][i],
        label,
        current_score: 0,
        max_possible: 100,
      }));

  // Calculate points on axes
  const getPoint = (index, value) => {
    const angle = startAngle + index * angleStep;
    const r = (value / 100) * maxRadius;
    return {
      x: cx + r * Math.cos(angle),
      y: cy + r * Math.sin(angle),
    };
  };

  // Axis endpoints
  const axisEndpoints = dimData.map((_, i) => getPoint(i, 100));

  // Data polygon points
  const dataPoints = dimData.map((d, i) => {
    const pct = d.max_possible > 0 ? (d.current_score / d.max_possible) * 100 : 0;
    return getPoint(i, pct);
  });
  const polygonPath = dataPoints.map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x},${p.y}`).join(' ') + ' Z';

  // Grid rings
  const ringLevels = [20, 40, 60, 80, 100];

  return (
    <div className="flex flex-col items-center" data-testid="odentity-polar-graph" role="img" aria-label={`0dentity polar score chart showing composite score of ${Math.round(compositeScore)} out of 100. See dimension breakdown below for text alternative.`}>
      <svg viewBox="0 0 400 400" width="400" className="w-full max-w-[400px]" style={{ overflow: 'visible' }} role="img" aria-label={`0dentity polar score chart showing composite score of ${Math.round(compositeScore)} out of 100`} aria-hidden="true">
        {/* Background circle */}
        <circle cx={cx} cy={cy} r={maxRadius} fill="#0F172A" stroke="#1E293B" strokeWidth="1" />

        {/* Grid rings */}
        {ringLevels.map(level => {
          const r = (level / 100) * maxRadius;
          return (
            <circle
              key={level}
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke="#1E293B"
              strokeWidth="0.5"
              strokeDasharray="3,3"
            />
          );
        })}

        {/* Axis lines */}
        {axisEndpoints.map((ep, i) => (
          <line
            key={`axis-${i}`}
            x1={cx}
            y1={cy}
            x2={ep.x}
            y2={ep.y}
            stroke="#334155"
            strokeWidth="1"
          />
        ))}

        {/* Data polygon - filled area */}
        <path
          d={polygonPath}
          fill="rgba(59, 130, 246, 0.2)"
          stroke="#3B82F6"
          strokeWidth="2"
          strokeLinejoin="round"
        />

        {/* Data points */}
        {dataPoints.map((p, i) => {
          const dim = dimData[i];
          const color = DIMENSION_COLORS[dim.dimension] || '#3B82F6';
          return (
            <circle
              key={`point-${i}`}
              cx={p.x}
              cy={p.y}
              r="5"
              fill={color}
              stroke="#fff"
              strokeWidth="1.5"
              style={onDimensionClick ? { cursor: 'pointer' } : {}}
              onClick={onDimensionClick ? () => onDimensionClick(dim) : undefined}
              data-testid={`dimension-point-${dim.dimension}`}
            />
          );
        })}

        {/* Axis labels */}
        {axisEndpoints.map((ep, i) => {
          const angle = startAngle + i * angleStep;
          const labelR = maxRadius + 25;
          const lx = cx + labelR * Math.cos(angle);
          const ly = cy + labelR * Math.sin(angle);

          // Determine text anchor based on position
          let textAnchor = 'middle';
          if (Math.cos(angle) < -0.1) textAnchor = 'end';
          else if (Math.cos(angle) > 0.1) textAnchor = 'start';

          // Shorten labels for display
          const shortLabels = [
            'Core Identity',
            'Medical Records',
            'PACE Trust',
            'Provider Trust',
            'Responder Access',
            'Credentials',
          ];

          const dim = dimData[i];
          const pct = dim.max_possible > 0 ? Math.round((dim.current_score / dim.max_possible) * 100) : 0;

          const isClickable = !!onDimensionClick;

          return (
            <g
              key={`label-${i}`}
              onClick={isClickable ? () => onDimensionClick(dim) : undefined}
              style={isClickable ? { cursor: 'pointer' } : {}}
              data-testid={`dimension-label-${dim.dimension}`}
            >
              {/* Invisible hit area for easier clicking */}
              {isClickable && (
                <rect
                  x={lx - 45}
                  y={ly - 20}
                  width="90"
                  height="36"
                  fill="transparent"
                />
              )}
              <text
                x={lx}
                y={ly - 6}
                textAnchor={textAnchor}
                fill={isClickable ? '#93C5FD' : '#CBD5E1'}
                fontSize="11"
                fontWeight="600"
                data-testid={`axis-label-${i}`}
                style={isClickable ? { textDecoration: 'underline' } : {}}
              >
                {shortLabels[i]}
              </text>
              <text
                x={lx}
                y={ly + 8}
                textAnchor={textAnchor}
                fill="#94A3B8"
                fontSize="10"
              >
                {pct}%
              </text>
            </g>
          );
        })}

        {/* Center composite score */}
        <text
          x={cx}
          y={cy - 8}
          textAnchor="middle"
          fill="#F1F5F9"
          fontSize="28"
          fontWeight="bold"
          data-testid="composite-score"
        >
          {Math.round(compositeScore)}
        </text>
        <text
          x={cx}
          y={cy + 12}
          textAnchor="middle"
          fill="#94A3B8"
          fontSize="11"
        >
          Composite Score
        </text>
      </svg>

      {/* Polygon area percentage */}
      <div className="mt-2 text-center" data-testid="polygon-area">
        <span className="text-sm text-gray-400">Polygon Area: </span>
        <span className="text-sm font-semibold text-sky-400">{polygonAreaPercentage}%</span>
      </div>

      {/* Legend */}
      <div className="mt-4 grid grid-cols-2 gap-2 w-full max-w-md">
        {dimData.map((dim, i) => {
          const color = DIMENSION_COLORS[dim.dimension] || '#3B82F6';
          const pct = dim.max_possible > 0 ? Math.round((dim.current_score / dim.max_possible) * 100) : 0;
          return (
            <div
              key={dim.dimension}
              className="flex items-center gap-2 text-xs"
              data-testid={`dimension-${dim.dimension}`}
            >
              <div
                className="w-3 h-3 rounded-full flex-shrink-0"
                style={{ backgroundColor: color }}
              />
              <span className="text-gray-300 truncate">{dim.label}</span>
              <span className="text-gray-500 ml-auto">{pct}%</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default OdentityPolarGraph;
