import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate } from 'react-router-dom';
import api from '../services/api';
import OdentityPolarGraph from '../components/OdentityPolarGraph';
import Navbar from '../components/Navbar';

const POLL_INTERVAL_MS = 10000; // Auto-refresh every 10 seconds
const SCORE_CACHE_KEY = 'odentity_score_cache';
const GATES_CACHE_KEY = 'odentity_gates_cache';
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes

function loadScoreCache() {
  try {
    const raw = sessionStorage.getItem(SCORE_CACHE_KEY);
    if (!raw) return null;
    const { data, timestamp } = JSON.parse(raw);
    if (Date.now() - timestamp > CACHE_TTL_MS) return null; // expired
    return data;
  } catch {
    return null;
  }
}

function loadGatesCache() {
  try {
    const raw = sessionStorage.getItem(GATES_CACHE_KEY);
    if (!raw) return null;
    const { data, timestamp } = JSON.parse(raw);
    if (Date.now() - timestamp > CACHE_TTL_MS) return null;
    return data;
  } catch {
    return null;
  }
}

function saveScoreCache(data) {
  try {
    sessionStorage.setItem(SCORE_CACHE_KEY, JSON.stringify({ data, timestamp: Date.now() }));
  } catch { /* ignore quota errors */ }
}

function saveGatesCache(data) {
  try {
    sessionStorage.setItem(GATES_CACHE_KEY, JSON.stringify({ data, timestamp: Date.now() }));
  } catch { /* ignore quota errors */ }
}

// Improvement opportunities per dimension
const DIMENSION_OPPORTUNITIES = {
  identity_core: {
    label: 'Core Identity',
    color: '#3B82F6',
    opportunities: [
      { claim: 'email_verified', label: 'Verify your email address', points: 10, link: '/settings', linkLabel: 'Go to Settings' },
      { claim: 'phone_verified', label: 'Verify your phone number', points: 10, link: '/settings', linkLabel: 'Go to Settings' },
      { claim: 'government_id_uploaded', label: 'Upload government ID (passport, driver\'s license)', points: 25, link: '/credentials', linkLabel: 'Upload Credentials' },
      { claim: 'profile_complete', label: 'Complete your subscriber profile', points: 15, link: '/profile', linkLabel: 'Update Profile' },
    ],
  },
  health_record_completeness: {
    label: 'Medical Record Completeness',
    color: '#10B981',
    opportunities: [
      { claim: 'allergy_added', label: 'Add allergy information', points: 5, link: '/profile', linkLabel: 'Update Health Info' },
      { claim: 'medication_added', label: 'Add current medications', points: 5, link: '/profile', linkLabel: 'Update Health Info' },
      { claim: 'condition_added', label: 'Add medical conditions', points: 5, link: '/profile', linkLabel: 'Update Health Info' },
      { claim: 'medical_record_uploaded', label: 'Upload a medical record', points: 10, link: '/health-vault', linkLabel: 'Go to Health Vault' },
      { claim: 'emergency_contact_added', label: 'Add emergency contacts', points: 5, link: '/profile', linkLabel: 'Update Profile' },
    ],
  },
  pace_trust_network: {
    label: 'PACE Trust Network',
    color: '#F59E0B',
    opportunities: [
      { claim: 'trustee_appointed', label: 'Appoint a PACE trustee', points: 10, link: '/pace', linkLabel: 'Manage PACE Trustees' },
      { claim: 'vss_ceremony_complete', label: 'Complete VSS key ceremony', points: 25, link: '/pace', linkLabel: 'Manage PACE Trustees' },
      { claim: 'trustee_accepted', label: 'Have trustees accept invitations', points: 10, link: '/pace', linkLabel: 'Manage PACE Trustees' },
      { claim: 'four_trustees_appointed', label: 'Appoint 4 PACE trustees (full network)', points: 20, link: '/pace', linkLabel: 'Manage PACE Trustees' },
    ],
  },
  provider_trust: {
    label: 'Provider Trust',
    color: '#8B5CF6',
    opportunities: [
      { claim: 'provider_linked', label: 'Grant access to a healthcare provider', points: 20, link: '/provider-access', linkLabel: 'Manage Provider Access' },
      { claim: 'provider_npi_verified', label: 'Use an NPI-verified provider', points: 15, link: '/provider-access', linkLabel: 'Manage Provider Access' },
      { claim: 'multiple_providers', label: 'Connect 3+ verified providers', points: 25, link: '/provider-access', linkLabel: 'Manage Provider Access' },
    ],
  },
  responder_accessibility: {
    label: 'First Responder Accessibility',
    color: '#EF4444',
    opportunities: [
      { claim: 'card_issued', label: 'Issue your emergency response card', points: 25, link: '/card', linkLabel: 'View My Card' },
      { claim: 'first_scan', label: 'Complete a test scan of your card', points: 15, link: '/scan-history', linkLabel: 'View Scan History' },
      { claim: 'allergies_emergency_visible', label: 'Set allergy info to emergency visible', points: 10, link: '/credentials', linkLabel: 'Manage Credentials' },
      { claim: 'insurance_emergency_visible', label: 'Set insurance card to emergency visible', points: 10, link: '/credentials', linkLabel: 'Manage Credentials' },
    ],
  },
  credential_issuers: {
    label: 'External Credential Issuers',
    color: '#EC4899',
    opportunities: [
      { claim: 'w3c_vc_imported', label: 'Import a W3C Verifiable Credential', points: 15, link: '/credentials', linkLabel: 'Manage Credentials' },
      { claim: 'open_badge_imported', label: 'Import an OpenBadge certification', points: 10, link: '/credentials', linkLabel: 'Manage Credentials' },
      { claim: 'eidas_imported', label: 'Import an eIDAS attestation', points: 15, link: '/credentials', linkLabel: 'Manage Credentials' },
      { claim: 'insurance_card', label: 'Add insurance card credential', points: 10, link: '/credentials', linkLabel: 'Manage Credentials' },
    ],
  },
};

function OdentityScore() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  // Initialize from cache for immediate display (lazy initializer runs only once)
  const [scoreData, setScoreData] = useState(() => loadScoreCache());
  const [gatedFeatures, setGatedFeatures] = useState(() => loadGatesCache() || []);
  const hasCachedDataRef = useRef(!!loadScoreCache());
  const [loading, setLoading] = useState(() => !loadScoreCache()); // no loading spinner if we have cache
  const [fromCache, setFromCache] = useState(() => !!loadScoreCache());
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState('');
  const [lastUpdated, setLastUpdated] = useState(null);
  const [selectedDimension, setSelectedDimension] = useState(null);
  const pollTimerRef = useRef(null);

  // Claims state
  const [claims, setClaims] = useState([]);
  const [claimsLoading, setClaimsLoading] = useState(false);
  const [claimDimensionFilter, setClaimDimensionFilter] = useState('all');

  // VC export state (Feature #387)
  const [exportingVC, setExportingVC] = useState(false);
  const [exportVCError, setExportVCError] = useState('');

  const fetchScore = useCallback(async (silent = false) => {
    try {
      if (!silent) setLoading(true);
      else setRefreshing(true);
      const [scoreRes, gatesRes] = await Promise.all([
        api.get('/odentity/me/score'),
        api.get('/odentity/me/gated-features'),
      ]);
      // Save to cache for next visit
      saveScoreCache(scoreRes.data);
      saveGatesCache(gatesRes.data.gated_features || []);
      setScoreData(scoreRes.data);
      setGatedFeatures(gatesRes.data.gated_features || []);
      setLastUpdated(new Date());
      setFromCache(false); // now showing live data
      setError('');
    } catch (err) {
      console.error('Failed to fetch 0dentity score:', err);
      if (!silent) setError('Failed to load 0dentity score');
    } finally {
      if (!silent) setLoading(false);
      else setRefreshing(false);
    }
  }, []);

  const fetchClaims = useCallback(async () => {
    try {
      setClaimsLoading(true);
      const res = await api.get('/odentity/me/claims');
      setClaims(res.data || []);
    } catch (err) {
      console.error('Failed to fetch claims:', err);
    } finally {
      setClaimsLoading(false);
    }
  }, []);

  // Initial load + auto-polling for real-time updates
  useEffect(() => {
    // If we have cached data, do a silent background refresh; otherwise show loading
    fetchScore(hasCachedDataRef.current);
    fetchClaims();

    // Poll every POLL_INTERVAL_MS for real-time graph updates
    pollTimerRef.current = setInterval(() => {
      fetchScore(true);
    }, POLL_INTERVAL_MS);

    return () => {
      if (pollTimerRef.current) clearInterval(pollTimerRef.current);
    };
  }, [fetchScore, fetchClaims]);

  const handleManualRefresh = () => {
    fetchScore(true);
  };

  const handleDimensionClick = (dim) => {
    setSelectedDimension(prev => prev && prev.dimension === dim.dimension ? null : dim);
  };

  // Feature #387: Export 0dentity claims as W3C Verifiable Credential JSON
  const handleExportVC = async () => {
    setExportingVC(true);
    setExportVCError('');
    try {
      const res = await api.get('/odentity/me/export-vc');
      const vc = res.data;
      // Download as JSON file
      const blob = new Blob([JSON.stringify(vc, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `livesafe-identity-credential-${new Date().toISOString().split('T')[0]}.json`;
      a.setAttribute('data-testid', 'vc-download-link');
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export VC:', err);
      setExportVCError('Failed to export credentials. Please try again.');
    } finally {
      setExportingVC(false);
    }
  };

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  return (
    <div className="min-h-screen bg-gray-950">
      <Navbar />

      <main id="main-content" tabIndex={-1} className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {loading ? (
          <div className="flex items-center justify-center py-20">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
              <p className="text-gray-400">Loading 0dentity score...</p>
            </div>
          </div>
        ) : error ? (
          <div className="bg-red-900/50 border border-red-700 rounded-lg p-4 text-red-300 text-center">
            {error}
          </div>
        ) : (
          <div className="space-y-8">
            {/* Score Header */}
            <div className="text-center">
              <h1 className="text-2xl font-bold text-white mb-1">Your 0dentity Score</h1>
              <p className="text-gray-400 text-sm">
                Multi-dimensional identity completeness across six trust dimensions
              </p>
              <div className="flex items-center justify-center gap-3 mt-2">
                {fromCache && (
                  <span className="text-xs text-amber-400 flex items-center gap-1" data-testid="cache-indicator">
                    📋 {refreshing ? 'Refreshing cached score...' : 'Showing cached score'}
                  </span>
                )}
                {lastUpdated && !fromCache && (
                  <span className="text-xs text-gray-500" data-testid="last-updated">
                    Last updated: {lastUpdated.toLocaleTimeString()}
                  </span>
                )}
                <button
                  onClick={handleManualRefresh}
                  disabled={refreshing}
                  className="text-xs text-sky-400 hover:text-sky-300 flex items-center gap-1 disabled:opacity-50"
                  title="Refresh score"
                  data-testid="refresh-score-btn"
                >
                  <span className={refreshing ? 'animate-spin inline-block' : ''}>{refreshing ? '⟳' : '↺'}</span>
                  {refreshing ? 'Refreshing...' : 'Refresh'}
                </button>
                <button
                  onClick={handleExportVC}
                  disabled={exportingVC}
                  className="text-xs px-3 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg flex items-center gap-1 disabled:opacity-50 transition-colors"
                  title="Export 0dentity claims as W3C Verifiable Credential"
                  data-testid="export-vc-btn"
                  aria-label="Export credentials as W3C Verifiable Credential"
                >
                  <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                  </svg>
                  {exportingVC ? 'Exporting...' : 'Export Credentials'}
                </button>
              </div>
              {exportVCError && (
                <p className="text-xs text-red-400 mt-2" role="alert" data-testid="export-vc-error">{exportVCError}</p>
              )}
            </div>

            {/* Polar Graph */}
            <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
              <p className="text-xs text-gray-500 text-center mb-3">💡 Click any dimension label or point to see improvement opportunities</p>
              <OdentityPolarGraph
                dimensions={scoreData?.dimensions || []}
                compositeScore={scoreData?.composite_score || 0}
                polygonAreaPercentage={scoreData?.polygon_area_percentage || 0}
                onDimensionClick={handleDimensionClick}
              />
            </div>

            {/* Dimension Improvement Panel - shown when a dimension is clicked */}
            {selectedDimension && (() => {
              const oppData = DIMENSION_OPPORTUNITIES[selectedDimension.dimension];
              const pct = selectedDimension.max_possible > 0
                ? Math.round((selectedDimension.current_score / selectedDimension.max_possible) * 100)
                : 0;
              const color = oppData?.color || '#3B82F6';
              return (
                <div
                  className="bg-gray-900 rounded-xl border-2 p-6"
                  style={{ borderColor: color }}
                  data-testid="dimension-improvement-panel"
                >
                  <div className="flex items-center justify-between mb-4">
                    <div className="flex items-center gap-3">
                      <div className="w-4 h-4 rounded-full" style={{ backgroundColor: color }} />
                      <h2 className="text-lg font-semibold text-white">
                        {selectedDimension.label} — Improvement Opportunities
                      </h2>
                    </div>
                    <div className="flex items-center gap-3">
                      <span className="text-sm font-mono" style={{ color }}>
                        {pct}% complete ({selectedDimension.current_score}/{selectedDimension.max_possible} pts)
                      </span>
                      <button
                        onClick={() => setSelectedDimension(null)}
                        className="text-gray-500 hover:text-gray-300 text-lg leading-none"
                        aria-label="Close improvement panel"
                      >
                        ✕
                      </button>
                    </div>
                  </div>
                  {oppData ? (
                    <div className="space-y-3">
                      {oppData.opportunities.map((opp) => (
                        <div
                          key={opp.claim}
                          className="flex items-center justify-between p-3 rounded-lg bg-gray-800/70 border border-gray-700"
                          data-testid={`opportunity-${opp.claim}`}
                        >
                          <div className="flex-1">
                            <p className="text-sm text-gray-200">{opp.label}</p>
                          </div>
                          <div className="flex items-center gap-3 ml-3 flex-shrink-0">
                            <span
                              className="text-xs font-semibold px-2 py-1 rounded-full"
                              style={{ backgroundColor: color + '30', color }}
                              data-testid={`opportunity-points-${opp.claim}`}
                            >
                              +{opp.points} pts
                            </span>
                            <button
                              onClick={() => navigate(opp.link)}
                              className="text-xs px-3 py-1.5 rounded-lg text-white font-medium hover:opacity-90 transition"
                              style={{ backgroundColor: color }}
                              data-testid={`opportunity-link-${opp.claim}`}
                            >
                              {opp.linkLabel}
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-gray-400 text-sm">No specific opportunities available for this dimension.</p>
                  )}
                </div>
              );
            })()}

            {/* Dimension Details — text/table alternative for polar graph (Feature #360) */}
            <div className="bg-gray-900 rounded-xl border border-gray-800 p-6" data-testid="score-text-alternative" role="region" aria-label="0dentity score dimensions text alternative">
              <h2 className="text-lg font-semibold text-white mb-4">Dimension Breakdown</h2>
              {/* Composite score summary for easy programmatic access */}
              <div className="flex justify-between items-center mb-3 p-2 rounded-lg bg-gray-800/50 border border-gray-700">
                <span className="text-sm text-gray-400 font-medium">Composite Score (weighted)</span>
                <span
                  className="text-sm font-bold text-sky-400 font-mono"
                  data-testid="composite-score-value"
                  data-composite-score={scoreData?.composite_score || 0}
                >
                  {Math.round(scoreData?.composite_score || 0)}
                </span>
              </div>
              <div className="space-y-3">
                {(scoreData?.dimensions || []).map((dim) => {
                  const pct = dim.max_possible > 0 ? Math.round((dim.current_score / dim.max_possible) * 100) : 0;
                  const weightPct = Math.round((dim.weight || 0) * 100);
                  const isSelected = selectedDimension && selectedDimension.dimension === dim.dimension;
                  return (
                    <div
                      key={dim.dimension}
                      className={`flex items-center gap-3 p-2 rounded-lg cursor-pointer transition-colors ${isSelected ? 'bg-gray-800' : 'hover:bg-gray-800/50'}`}
                      onClick={() => handleDimensionClick(dim)}
                      title="Click to see improvement opportunities"
                      data-testid={`dimension-row-${dim.dimension}`}
                      data-current-score={dim.current_score}
                      data-max-possible={dim.max_possible}
                      data-weight={dim.weight}
                    >
                      <div className="flex-1">
                        <div className="flex justify-between mb-1">
                          <span className="text-sm text-blue-400 hover:text-blue-300">{dim.label}</span>
                          <span className="text-sm text-gray-500">
                            {dim.current_score}/{dim.max_possible} ({weightPct}% weight)
                          </span>
                        </div>
                        <div className="w-full bg-gray-800 rounded-full h-2">
                          <div
                            className="bg-sky-500 h-2 rounded-full transition-all"
                            style={{ width: `${pct}%` }}
                          />
                        </div>
                      </div>
                      <span className="text-sm font-mono text-sky-400 w-12 text-right">{pct}%</span>
                    </div>
                  );
                })}
              </div>
              <p className="text-xs text-gray-600 mt-3">Click any dimension to view improvement opportunities</p>
            </div>

            {/* Claims List with Dimension Filter */}
            <div className="bg-gray-900 rounded-xl border border-gray-800 p-6" data-testid="claims-section">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-white">My Claims</h2>
                <span className="text-xs text-gray-500">{claims.length} total claim{claims.length !== 1 ? 's' : ''}</span>
              </div>

              {/* Dimension Filter Buttons */}
              <div className="flex flex-wrap gap-2 mb-4" data-testid="claim-dimension-filter">
                <button
                  onClick={() => setClaimDimensionFilter('all')}
                  data-testid="claim-filter-all"
                  className={`px-3 py-1.5 rounded-full text-xs font-medium transition-colors ${
                    claimDimensionFilter === 'all'
                      ? 'bg-sky-500 text-white'
                      : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
                  }`}
                >
                  All ({claims.length})
                </button>
                {Object.entries(DIMENSION_OPPORTUNITIES).map(([dimKey, dimData]) => {
                  const count = claims.filter(c => c.dimension === dimKey).length;
                  return (
                    <button
                      key={dimKey}
                      onClick={() => setClaimDimensionFilter(dimKey)}
                      data-testid={`claim-filter-${dimKey}`}
                      className={`px-3 py-1.5 rounded-full text-xs font-medium transition-colors ${
                        claimDimensionFilter === dimKey
                          ? 'text-white'
                          : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
                      }`}
                      style={claimDimensionFilter === dimKey ? { backgroundColor: dimData.color } : {}}
                    >
                      {dimData.label} ({count})
                    </button>
                  );
                })}
              </div>

              {/* Filtered Claims List */}
              {claimsLoading ? (
                <div className="text-center py-6">
                  <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-sky-500 mx-auto mb-2"></div>
                  <p className="text-gray-400 text-sm">Loading claims...</p>
                </div>
              ) : (() => {
                const filtered = claimDimensionFilter === 'all'
                  ? claims
                  : claims.filter(c => c.dimension === claimDimensionFilter);
                if (filtered.length === 0) {
                  return (
                    <div className="text-center py-6 text-gray-500 text-sm" data-testid="claims-empty">
                      {claimDimensionFilter === 'all'
                        ? 'No claims yet. Complete actions to earn claims and improve your score.'
                        : `No claims in the ${DIMENSION_OPPORTUNITIES[claimDimensionFilter]?.label || claimDimensionFilter} dimension yet.`
                      }
                    </div>
                  );
                }
                return (
                  <div className="space-y-2" data-testid="claims-list">
                    {filtered.map((claim) => {
                      const dimData = DIMENSION_OPPORTUNITIES[claim.dimension];
                      const color = dimData?.color || '#6B7280';
                      return (
                        <div
                          key={claim.id}
                          className="flex items-center justify-between p-3 rounded-lg bg-gray-800/70 border border-gray-700"
                          data-testid={`claim-item-${claim.id}`}
                          data-dimension={claim.dimension}
                        >
                          <div className="flex items-center gap-3 min-w-0">
                            <span
                              className="flex-shrink-0 text-xs font-medium px-2 py-0.5 rounded-full"
                              style={{ backgroundColor: color + '30', color }}
                              data-testid={`claim-dimension-badge-${claim.id}`}
                            >
                              {dimData?.label || claim.dimension}
                            </span>
                            <span className="text-sm text-gray-200 truncate" data-testid={`claim-type-${claim.id}`}>
                              {claim.claim_type.replace(/_/g, ' ')}
                            </span>
                            {claim.revoked_at && (
                              <span className="flex-shrink-0 text-xs text-red-400 bg-red-900/30 px-2 py-0.5 rounded-full">
                                Revoked
                              </span>
                            )}
                          </div>
                          <div className="flex items-center gap-3 flex-shrink-0 ml-3">
                            <span
                              className="text-xs font-semibold font-mono"
                              style={{ color }}
                              data-testid={`claim-points-${claim.id}`}
                            >
                              +{parseFloat(claim.points_awarded).toFixed(0)} pts
                            </span>
                            <span className="text-xs text-gray-500">
                              {new Date(claim.issued_at).toLocaleDateString()}
                            </span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                );
              })()}
            </div>

            {/* Score-Gated Features */}
            <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
              <h2 className="text-lg font-semibold text-white mb-4">Score-Gated Features</h2>
              <div className="space-y-2">
                {gatedFeatures.map((gate) => (
                  <div
                    key={gate.feature}
                    className={`flex items-center justify-between p-3 rounded-lg border ${
                      gate.unlocked
                        ? 'border-green-700 bg-green-900/20'
                        : 'border-gray-700 bg-gray-800/50'
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <span className={`text-lg ${gate.unlocked ? 'text-green-400' : 'text-gray-600'}`}>
                        {gate.unlocked ? '\u2713' : '\u2717'}
                      </span>
                      <span className={`text-sm ${gate.unlocked ? 'text-green-300' : 'text-gray-400'}`}>
                        {gate.label}
                      </span>
                    </div>
                    <span className={`text-xs font-mono ${gate.unlocked ? 'text-green-400' : 'text-gray-500'}`}>
                      {gate.score_minimum}+ required
                    </span>
                  </div>
                ))}
              </div>
            </div>

            {/* How to Improve */}
            <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
              <h2 className="text-lg font-semibold text-white mb-4">How to Improve Your Score</h2>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
                <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
                  <p className="text-blue-400 font-medium mb-1">Core Identity</p>
                  <p className="text-gray-400">Verify your email, phone, and government ID</p>
                </div>
                <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
                  <p className="text-emerald-400 font-medium mb-1">Medical Records</p>
                  <p className="text-gray-400">Complete your profile, add allergies, medications & conditions</p>
                </div>
                <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
                  <p className="text-amber-400 font-medium mb-1">PACE Trust Network</p>
                  <p className="text-gray-400">Appoint 4 PACE trustees and verify key shards</p>
                </div>
                <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
                  <p className="text-purple-400 font-medium mb-1">Provider Trust</p>
                  <p className="text-gray-400">Link healthcare providers and verify affiliations</p>
                </div>
                <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
                  <p className="text-red-400 font-medium mb-1">Responder Accessibility</p>
                  <p className="text-gray-400">Issue your emergency card and test a scan</p>
                </div>
                <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
                  <p className="text-pink-400 font-medium mb-1">External Credentials</p>
                  <p className="text-gray-400">Import W3C VCs, OpenBadges, or eIDAS attestations</p>
                </div>
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}

export default OdentityScore;
