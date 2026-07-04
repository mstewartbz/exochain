import React, { useState, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';

const SCOPE_OPTIONS = [
  { value: 'full_medical_record', label: 'Full Medical Record' },
  { value: 'emergency_info', label: 'Emergency Information Only' },
  { value: 'allergies_medications', label: 'Allergies & Medications' },
  { value: 'lab_results', label: 'Lab Results' },
  { value: 'imaging', label: 'Imaging / X-Ray Records' },
  { value: 'conditions', label: 'Medical Conditions' },
  { value: 'prescriptions', label: 'Prescriptions' },
];

const PURPOSE_OPTIONS = [
  { value: 'ongoing_medical_care', label: 'Ongoing Medical Care' },
  { value: 'specialist_referral', label: 'Specialist Referral' },
  { value: 'emergency_treatment', label: 'Emergency Treatment' },
  { value: 'second_opinion', label: 'Second Opinion' },
  { value: 'insurance_claim', label: 'Insurance Claim' },
];

const DURATION_OPTIONS = [
  { value: '24', label: '24 Hours' },
  { value: '168', label: '1 Week' },
  { value: '720', label: '30 Days' },
  { value: '2160', label: '90 Days' },
  { value: '8760', label: '1 Year' },
  { value: '', label: 'No Expiration' },
];

export default function ProviderAccess() {
  const { user } = useAuth();
  const navigate = useNavigate();

  const [providers, setProviders] = useState([]);
  const [consents, setConsents] = useState([]);
  const [accessRequests, setAccessRequests] = useState([]);
  const [loading, setLoading] = useState(true);

  // Feature #133/#134: 0dentity score and gated features
  const [odentityScore, setOdentityScore] = useState(null);
  const [gatedFeatures, setGatedFeatures] = useState([]);

  // Provider list filter state (Feature #218)
  const [providerSpecialtyFilter, setProviderSpecialtyFilter] = useState('all');
  const [providerFacilityFilter, setProviderFacilityFilter] = useState('all');

  // Feature #268: Idempotency guard - prevents concurrent/double grant submissions
  const isGrantingRef = useRef(false);

  // Feature #272: Idempotency guard - prevents concurrent/double revoke submissions
  const [revokingIds, setRevokingIds] = useState(new Set());

  // Feature #296: Error message on consent revocation failure
  const [revokeError, setRevokeError] = useState('');

  // Provider search state (Feature #249: state preserved during provider search)
  const [providerSearchText, setProviderSearchText] = useState('');
  const [providerSearchLoading, setProviderSearchLoading] = useState(false);
  const [searchedProviders, setSearchedProviders] = useState(null); // null = show all, array = search results
  const providerSearchTimerRef = useRef(null);
  // Feature #378: AbortController ref for stale response prevention in provider search
  const providerSearchAbortRef = useRef(null);

  // Grant form state
  const [selectedProvider, setSelectedProvider] = useState('');
  const [scope, setScope] = useState('');
  const [purpose, setPurpose] = useState('ongoing_medical_care');
  const [durationHours, setDurationHours] = useState('720');
  const [durationDays, setDurationDays] = useState('30');
  const [useCustomDuration, setUseCustomDuration] = useState(false);
  const [granting, setGranting] = useState(false);
  const [grantMessage, setGrantMessage] = useState('');
  const [grantError, setGrantError] = useState('');

  // Feature #366: Live countdown ticker - updates every minute to keep remaining time accurate
  const [now, setNow] = useState(new Date());
  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), 60000);
    return () => clearInterval(timer);
  }, []);

  // Feature #366: Format remaining time for time-limited consents (countdown)
  const formatRemainingTime = (expiresAt) => {
    if (!expiresAt) return null;
    const exp = new Date(expiresAt);
    const diffMs = exp - now;
    if (diffMs <= 0) return 'Expired';
    const totalSecs = Math.floor(diffMs / 1000);
    const days = Math.floor(totalSecs / 86400);
    const hours = Math.floor((totalSecs % 86400) / 3600);
    const minutes = Math.floor((totalSecs % 3600) / 60);
    if (days > 0) return `${days}d ${hours}h remaining`;
    if (hours > 0) return `${hours}h ${minutes}m remaining`;
    return `${minutes}m remaining`;
  };

  // Format a UTC timestamp as local datetime string with explicit timezone label
  const formatLocalDateTime = (utcTimestamp) => {
    if (!utcTimestamp) return 'N/A';
    const d = new Date(utcTimestamp);
    // toLocaleString() uses user's system timezone automatically (local timezone)
    return d.toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZoneName: 'short',
    });
  };

  // Access request response state
  const [respondingId, setRespondingId] = useState(null);
  const [requestResponseMsg, setRequestResponseMsg] = useState('');

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      setLoading(true);
      const [providersRes, consentsRes, accessReqRes, scoreRes, gatesRes] = await Promise.all([
        api.get('/consent/providers'),
        api.get('/consent/my-consents'),
        api.get('/consent/access-requests').catch(() => ({ data: [] })),
        api.get('/odentity/me/score').catch(() => ({ data: null })),
        api.get('/odentity/me/gated-features').catch(() => ({ data: { gated_features: [] } })),
      ]);
      setProviders(providersRes.data);
      setConsents(consentsRes.data);
      setAccessRequests(accessReqRes.data || []);
      setOdentityScore(scoreRes.data?.composite_score ?? null);
      setGatedFeatures(gatesRes.data?.gated_features || []);

      // Feature #104: Check for recently expired consents and notify subscriber
      api.get('/consent/expiry-check').catch(() => {});
    } catch (err) {
      console.error('Failed to load data:', err);
    } finally {
      setLoading(false);
    }
  };

  // Feature #249/#378: Async provider search - scope/purpose/duration state preserved during search
  // Feature #378: Debouncing (400ms) prevents excessive requests on rapid typing.
  // AbortController cancels stale in-flight requests so no stale results can overwrite fresh ones.
  const handleProviderSearch = (text) => {
    setProviderSearchText(text);

    // Clear previous debounce timer
    if (providerSearchTimerRef.current) {
      clearTimeout(providerSearchTimerRef.current);
    }

    // Feature #378: Cancel any in-flight search request that is now stale
    if (providerSearchAbortRef.current) {
      providerSearchAbortRef.current.abort();
      providerSearchAbortRef.current = null;
    }

    if (!text.trim()) {
      setSearchedProviders(null);
      setProviderSearchLoading(false);
      return;
    }

    // Debounce: wait 400ms before searching to prevent excessive API requests
    setProviderSearchLoading(true);
    providerSearchTimerRef.current = setTimeout(async () => {
      // Feature #378: Create new AbortController for this request
      const controller = new AbortController();
      providerSearchAbortRef.current = controller;
      try {
        // NOTE: scope/purpose/duration state are NOT touched here - preserved by design
        const res = await api.get(`/consent/providers?search=${encodeURIComponent(text.trim())}`, {
          signal: controller.signal,
        });
        // Only update state if this request was not cancelled by a newer one
        if (!controller.signal.aborted) {
          setSearchedProviders(res.data);
        }
      } catch (err) {
        // Ignore abort errors — they are intentional cancellations, not real failures
        if (err.name === 'CanceledError' || err.name === 'AbortError' || err.code === 'ERR_CANCELED') {
          return;
        }
        console.error('[ProviderSearch] Search failed:', err.message);
        setSearchedProviders([]);
      } finally {
        // Only clear loading if this is still the current request
        if (providerSearchAbortRef.current === controller) {
          setProviderSearchLoading(false);
          providerSearchAbortRef.current = null;
        }
      }
    }, 400);
  };

  const handleApproveRequest = async (requestId) => {
    setRespondingId(requestId);
    setRequestResponseMsg('');
    try {
      await api.post(`/consent/access-requests/${requestId}/approve`, { duration_hours: 720 });
      setRequestResponseMsg('Access request approved. Provider notified.');
      await loadData();
      setTimeout(() => setRequestResponseMsg(''), 5000);
    } catch (err) {
      setRequestResponseMsg(err.response?.data?.error || 'Failed to approve request');
    } finally {
      setRespondingId(null);
    }
  };

  const handleDenyRequest = async (requestId) => {
    setRespondingId(requestId);
    setRequestResponseMsg('');
    try {
      await api.post(`/consent/access-requests/${requestId}/deny`);
      setRequestResponseMsg('Access request denied.');
      await loadData();
      setTimeout(() => setRequestResponseMsg(''), 5000);
    } catch (err) {
      setRequestResponseMsg(err.response?.data?.error || 'Failed to deny request');
    } finally {
      setRespondingId(null);
    }
  };

  const handleGrant = async (e) => {
    e.preventDefault();

    // Feature #268: Idempotency guard - prevent concurrent/double grant submissions (useRef is synchronous)
    if (isGrantingRef.current) return;

    if (!selectedProvider) {
      setGrantError('Please select a provider');
      return;
    }
    if (!scope) {
      setGrantError('Please select an access scope');
      return;
    }

    // Validate duration if custom mode
    let finalDurationHours = durationHours || null;
    if (useCustomDuration) {
      const days = parseFloat(durationDays);
      if (isNaN(days) || days <= 0) {
        setGrantError('Access duration must be a positive number of days');
        return;
      }
      finalDurationHours = String(Math.round(days * 24));
    } else if (durationHours !== '') {
      const hours = parseInt(durationHours);
      if (isNaN(hours) || hours <= 0) {
        setGrantError('Access duration must be a positive value');
        return;
      }
    }

    isGrantingRef.current = true;
    setGranting(true);
    setGrantMessage('');
    setGrantError('');

    try {
      const res = await api.post('/consent/grant', {
        provider_id: parseInt(selectedProvider),
        scope,
        purpose,
        duration_hours: finalDurationHours || null,
      });

      if (res.data.idempotent) {
        // Feature #268: Idempotent case - back-and-resubmit: existing consent returned, no duplicate
        setGrantMessage(`Provider already has active access (no duplicate created). Consent ID: ${res.data.consent.id}`);
      } else {
        setGrantMessage(`Access granted to ${res.data.consent.provider_name || 'provider'}. Audit receipt: ${res.data.audit_receipt}`);
      }
      setSelectedProvider('');
      setScope('');
      setPurpose('ongoing_medical_care');
      setDurationHours('720');
      setDurationDays('30');
      setUseCustomDuration(false);

      // Reload consents
      const consentsRes = await api.get('/consent/my-consents');
      setConsents(consentsRes.data);

      setTimeout(() => setGrantMessage(''), 5000);
    } catch (err) {
      setGrantError(err.response?.data?.error || 'Failed to grant access');
    } finally {
      setGranting(false);
      isGrantingRef.current = false;
    }
  };

  const handleRevoke = async (consentId) => {
    // Feature #272: Prevent concurrent/double revocations
    if (revokingIds.has(consentId)) return;
    if (!window.confirm('Are you sure you want to revoke this access?')) return;

    setRevokeError(''); // Feature #296: clear previous error
    setRevokingIds(prev => new Set([...prev, consentId]));
    try {
      const res = await api.delete(`/consent/${consentId}`);
      // Handle graceful already-revoked response
      if (res.data?.already_revoked) {
        console.log(`Consent #${consentId} was already revoked`);
      }
      const consentsRes = await api.get('/consent/my-consents');
      setConsents(consentsRes.data);
    } catch (err) {
      console.error('Failed to revoke consent:', err);
      // Feature #296: Show user-friendly error message
      const msg = err.response?.data?.error || 'Failed to revoke consent. Please try again.';
      setRevokeError(msg);
      setTimeout(() => setRevokeError(''), 8000);
    } finally {
      setRevokingIds(prev => {
        const next = new Set(prev);
        next.delete(consentId);
        return next;
      });
    }
  };

  const isActive = (consent) => {
    if (consent.revoked_at) return false;
    if (consent.expires_at && new Date(consent.expires_at) < new Date()) return false;
    return true;
  };

  const formatExpiry = (consent) => {
    if (consent.revoked_at) return 'Revoked';
    if (!consent.expires_at) return 'No expiration';
    const exp = new Date(consent.expires_at);
    if (exp < new Date()) return 'Expired';
    return `Expires ${exp.toLocaleString(undefined, { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', timeZoneName: 'short' })}`;
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500"></div>
      </div>
    );
  }

  const activeConsents = consents.filter(isActive);
  const inactiveConsents = consents.filter(c => !isActive(c));

  return (
    <div className="min-h-screen bg-gray-50">
      <nav className="bg-white shadow-sm border-b">
        <div className="max-w-4xl mx-auto px-4 py-3 flex justify-between items-center">
          <div className="flex items-center gap-2">
            <span className="text-sky-700 font-bold text-xl cursor-pointer" onClick={() => navigate('/dashboard')}>
              LiveSafe<span className="text-emerald-600">.ai</span>
            </span>
            <span className="text-gray-400">›</span>
            <span className="text-sm font-medium text-gray-700">Provider Access</span>
          </div>
          <button onClick={() => navigate('/dashboard')} className="text-sky-600 hover:text-sky-700 text-sm">
            ← Back to Dashboard
          </button>
        </div>
      </nav>

      <main className="max-w-4xl mx-auto px-4 py-6">
        <h1 className="text-2xl font-bold text-gray-900 mb-6">Provider Access Management</h1>

        {/* Pending Provider Access Requests (Feature #103) */}
        {accessRequests.filter(r => r.status === 'pending').length > 0 && (
          <div className="bg-amber-50 border border-amber-200 rounded-lg p-6 mb-6" data-testid="pending-access-requests">
            <h2 className="text-lg font-semibold text-amber-900 mb-1">
              📋 Pending Access Requests ({accessRequests.filter(r => r.status === 'pending').length})
            </h2>
            <p className="text-sm text-amber-700 mb-4">
              Providers are requesting expanded access to your health data. Review and approve or deny below.
            </p>

            {requestResponseMsg && (
              <div className="mb-3 p-3 bg-white border border-amber-300 text-amber-800 rounded text-sm" data-testid="access-request-response-msg">
                {requestResponseMsg}
              </div>
            )}

            <div className="space-y-3">
              {accessRequests.filter(r => r.status === 'pending').map(req => (
                <div key={req.id} className="bg-white border border-amber-200 rounded-lg p-4" data-testid={`access-request-${req.id}`}>
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <p className="font-medium text-gray-900">
                        {req.provider_name || req.provider_email} — requesting <strong>{req.requested_scope}</strong>
                      </p>
                      {req.facility && <p className="text-xs text-gray-500 mt-0.5">{req.facility} · NPI: {req.npi}</p>}
                      {req.message && (
                        <p className="text-sm text-gray-600 mt-1 italic">"{req.message}"</p>
                      )}
                      <p className="text-xs text-gray-400 mt-1">
                        Requested: {formatLocalDateTime(req.requested_at)}
                      </p>
                    </div>
                    <div className="flex gap-2 ml-4">
                      <button
                        onClick={() => handleApproveRequest(req.id)}
                        disabled={respondingId === req.id}
                        className="px-3 py-1.5 text-sm font-medium bg-green-500 hover:bg-green-600 text-white rounded-lg transition disabled:opacity-50"
                        data-testid={`approve-request-${req.id}`}
                      >
                        {respondingId === req.id ? '...' : 'Approve'}
                      </button>
                      <button
                        onClick={() => handleDenyRequest(req.id)}
                        disabled={respondingId === req.id}
                        className="px-3 py-1.5 text-sm font-medium bg-red-500 hover:bg-red-600 text-white rounded-lg transition disabled:opacity-50"
                        data-testid={`deny-request-${req.id}`}
                      >
                        {respondingId === req.id ? '...' : 'Deny'}
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Feature #133: Score Gate Warning for Provider Sharing */}
        {odentityScore !== null && odentityScore < 25 && (
          <div className="bg-amber-50 border border-amber-300 rounded-lg p-5 mb-6" data-testid="score-gate-provider-sharing">
            <div className="flex items-start gap-3">
              <span className="text-2xl">🔒</span>
              <div className="flex-1">
                <h3 className="font-semibold text-amber-900 text-base mb-1">Provider Sharing Locked</h3>
                <p className="text-sm text-amber-800 mb-2">
                  Sharing your records with providers requires a <strong>0dentity score of 25 or higher</strong>.
                  Your current score is <strong>{odentityScore.toFixed(1)}</strong>.
                </p>
                <div className="flex items-center gap-2 mb-3">
                  <div className="flex-1 bg-amber-200 rounded-full h-2">
                    <div
                      className="bg-amber-500 h-2 rounded-full transition-all"
                      style={{ width: `${Math.min(100, (odentityScore / 25) * 100)}%` }}
                    />
                  </div>
                  <span className="text-xs font-mono text-amber-700">{odentityScore.toFixed(1)} / 25</span>
                </div>
                <p className="text-xs text-amber-700">
                  To unlock: complete your profile, verify your identity, and add medical records.
                  <button
                    className="ml-2 underline font-medium hover:text-amber-900"
                    onClick={() => navigate('/odentity')}
                  >
                    View your 0dentity Score →
                  </button>
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Feature #218/#249: Browse Verified Providers with specialty/facility filters + text search */}
        {providers.length > 0 && (() => {
          // Feature #249: Use searchedProviders when a search is active, else use all providers
          const baseProviders = (providerSearchText.trim() && searchedProviders !== null)
            ? searchedProviders
            : providers;
          const specialties = ['all', ...Array.from(new Set(providers.map(p => p.specialty).filter(Boolean))).sort()];
          const facilities = ['all', ...Array.from(new Set(providers.map(p => p.facility).filter(Boolean))).sort()];
          const filteredProviders = baseProviders.filter(p => {
            if (providerSpecialtyFilter !== 'all' && p.specialty !== providerSpecialtyFilter) return false;
            if (providerFacilityFilter !== 'all' && p.facility !== providerFacilityFilter) return false;
            return true;
          });
          return (
            <div className="bg-white rounded-lg shadow p-6 mb-6" data-testid="provider-list-section">
              <h2 className="text-lg font-semibold text-gray-900 mb-2">Browse Verified Providers</h2>
              <p className="text-sm text-gray-600 mb-4">
                Search or filter providers by specialty or facility to find the right provider for your care.
              </p>

              {/* Feature #249: Provider text search - state preserved during async search */}
              <div className="mb-4">
                <label className="block text-xs font-medium text-gray-600 mb-1">Search Providers</label>
                <div className="relative">
                  <input
                    type="text"
                    value={providerSearchText}
                    onChange={e => handleProviderSearch(e.target.value)}
                    placeholder="Search by name, NPI, facility, or specialty..."
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 pr-8 text-sm focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                    data-testid="provider-search"
                  />
                  {providerSearchLoading && (
                    <div className="absolute right-2 top-2" data-testid="provider-search-loading">
                      <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-sky-500"></div>
                    </div>
                  )}
                  {!providerSearchLoading && providerSearchText && (
                    <button
                      onClick={() => { setProviderSearchText(''); setSearchedProviders(null); }}
                      className="absolute right-2 top-2 text-gray-400 hover:text-gray-600 text-xs"
                      aria-label="Clear search"
                      data-testid="clear-provider-search"
                    >
                      ✕
                    </button>
                  )}
                </div>
                {providerSearchLoading && (
                  <p className="text-xs text-sky-700 mt-1" data-testid="provider-search-status">
                    Searching providers...
                    {/* Scope selections in the form below are preserved during this search */}
                  </p>
                )}
                {!providerSearchLoading && providerSearchText.trim() && searchedProviders !== null && (
                  <p className="text-xs text-gray-500 mt-1" data-testid="provider-search-result-count">
                    Found {searchedProviders.length} provider{searchedProviders.length !== 1 ? 's' : ''} matching "{providerSearchText}"
                  </p>
                )}
              </div>

              <div className="flex flex-col sm:flex-row gap-3 mb-4">
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1">Filter by Specialty</label>
                  <select
                    value={providerSpecialtyFilter}
                    onChange={e => setProviderSpecialtyFilter(e.target.value)}
                    className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500"
                    data-testid="specialty-filter"
                  >
                    {specialties.map(s => (
                      <option key={s} value={s}>{s === 'all' ? 'All Specialties' : s}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1">Filter by Facility</label>
                  <select
                    value={providerFacilityFilter}
                    onChange={e => setProviderFacilityFilter(e.target.value)}
                    className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500"
                    data-testid="facility-filter"
                  >
                    {facilities.map(f => (
                      <option key={f} value={f}>{f === 'all' ? 'All Facilities' : f}</option>
                    ))}
                  </select>
                </div>
                {(providerSpecialtyFilter !== 'all' || providerFacilityFilter !== 'all') && (
                  <div className="flex items-end">
                    <button
                      onClick={() => { setProviderSpecialtyFilter('all'); setProviderFacilityFilter('all'); }}
                      className="px-3 py-2 text-sm text-red-600 border border-red-200 rounded-lg hover:bg-red-50 transition"
                      data-testid="clear-provider-filters-btn"
                    >
                      Clear Filters
                    </button>
                  </div>
                )}
              </div>
              <div className="text-xs text-gray-500 mb-3" data-testid="provider-filter-count">
                Showing {filteredProviders.length} of {providers.length} provider{providers.length !== 1 ? 's' : ''}
                {(providerSpecialtyFilter !== 'all' || providerFacilityFilter !== 'all') && (
                  <span className="ml-2 text-sky-700 font-medium">• Filters active</span>
                )}
              </div>
              {filteredProviders.length === 0 ? (
                <p className="text-sm text-gray-500 py-4 text-center" data-testid="no-providers-found">
                  No providers match the selected filters{providerSearchText ? ` or search "${providerSearchText}"` : ''}.
                </p>
              ) : (
                <div className="space-y-2" data-testid="filtered-provider-list">
                  {filteredProviders.map(p => (
                    <div
                      key={p.id}
                      className="flex items-center justify-between p-3 border border-gray-200 rounded-lg hover:bg-gray-50"
                      data-testid={`provider-item-${p.id}`}
                    >
                      <div>
                        <p className="font-medium text-sm text-gray-900">{p.provider_name || p.email}</p>
                        <p className="text-xs text-gray-500 mt-0.5">
                          {p.specialty && <span className="mr-2" data-testid={`provider-specialty-${p.id}`}>🏥 {p.specialty}</span>}
                          {p.facility && <span data-testid={`provider-facility-${p.id}`}>📍 {p.facility}</span>}
                        </p>
                        <p className="text-xs text-gray-400">NPI: {p.npi}</p>
                      </div>
                      <button
                        onClick={() => {
                          setSelectedProvider(String(p.id));
                          document.querySelector('[data-testid="grant-button"]')?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
                        }}
                        className="text-xs px-3 py-1.5 bg-sky-100 text-sky-700 hover:bg-sky-200 rounded-lg transition font-medium"
                        data-testid={`select-provider-${p.id}`}
                      >
                        Select
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })()}

        {/* Grant Access Form */}
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Grant Provider Access</h2>
          <p className="text-sm text-gray-600 mb-4">
            Select a verified provider and define the scope of data they can access. Consent events stay in a local audit trail while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.
          </p>

          {grantMessage && (
            <div className="mb-4 p-3 rounded-lg text-sm bg-green-100 text-green-700" data-testid="grant-success">
              {grantMessage}
            </div>
          )}
          {grantError && (
            <div className="mb-4 p-3 rounded-lg text-sm bg-red-100 text-red-700" data-testid="grant-error">
              {grantError}
            </div>
          )}

          <form onSubmit={handleGrant} className="space-y-4">
            {/* Provider Selection */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Select Provider *</label>
              <select
                value={selectedProvider}
                onChange={e => setSelectedProvider(e.target.value)}
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                data-testid="provider-select"
              >
                <option value="">Choose a verified provider...</option>
                {providers.map(p => (
                  <option key={p.id} value={p.id}>
                    {p.provider_name || p.email} — {p.facility || 'No facility'} (NPI: {p.npi})
                  </option>
                ))}
              </select>
            </div>

            {/* Scope */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Access Scope *</label>
              <select
                value={scope}
                onChange={e => setScope(e.target.value)}
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                data-testid="scope-select"
              >
                <option value="">Choose scope of access...</option>
                {SCOPE_OPTIONS.map(s => (
                  <option key={s.value} value={s.value}>{s.label}</option>
                ))}
              </select>
              {/* Feature #134: Sovereignty scope warning */}
              {scope === 'full_medical_record' && odentityScore !== null && odentityScore < 75 && (
                <div className="mt-2 p-2 bg-amber-50 border border-amber-200 rounded text-xs text-amber-700" data-testid="sovereignty-scope-warning">
                  <strong>⚠️ Full Medical Sovereignty Required:</strong> Sharing "Full Medical Record" requires a 0dentity score of 75+.
                  Your current score is <strong>{odentityScore.toFixed(1)}</strong> / 75.
                </div>
              )}
            </div>

            {/* Purpose */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Purpose</label>
              <select
                value={purpose}
                onChange={e => setPurpose(e.target.value)}
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                data-testid="purpose-select"
              >
                {PURPOSE_OPTIONS.map(p => (
                  <option key={p.value} value={p.value}>{p.label}</option>
                ))}
              </select>
            </div>

            {/* Duration */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Access Duration</label>
              <select
                value={useCustomDuration ? 'custom' : durationHours}
                onChange={e => {
                  if (e.target.value === 'custom') {
                    setUseCustomDuration(true);
                    setDurationDays('30');
                  } else {
                    setUseCustomDuration(false);
                    setDurationHours(e.target.value);
                  }
                }}
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                data-testid="duration-select"
              >
                {DURATION_OPTIONS.map(d => (
                  <option key={d.value} value={d.value}>{d.label}</option>
                ))}
                <option value="custom">Custom (days)</option>
              </select>
              {useCustomDuration && (
                <div className="mt-2">
                  <input
                    type="number"
                    value={durationDays}
                    onChange={e => setDurationDays(e.target.value)}
                    placeholder="Enter number of days"
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                    data-testid="duration-days-input"
                  />
                  <p className="text-xs text-gray-500 mt-1">Enter a positive number of days (e.g., 30)</p>
                </div>
              )}
            </div>

            <button
              type="submit"
              disabled={
                granting ||
                (odentityScore !== null && odentityScore < 25) ||
                (odentityScore !== null && odentityScore < 75 && ['full_medical_record', 'full_medical_jacket'].includes(scope))
              }
              className="bg-sky-500 hover:bg-sky-600 text-white font-medium py-2 px-6 rounded-lg transition-colors disabled:opacity-50"
              data-testid="grant-button"
            >
              {granting ? 'Granting Access...' :
               odentityScore !== null && odentityScore < 25 ? '🔒 Score Too Low (Need 25+)' :
               odentityScore !== null && odentityScore < 75 && ['full_medical_record', 'full_medical_jacket'].includes(scope) ? '🔒 Sovereignty Score Too Low (Need 75+)' :
               'Grant Access'}
            </button>
          </form>
        </div>

        {/* Active Consents */}
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            Active Access Grants ({activeConsents.length})
          </h2>

          {/* Feature #296: Revocation error message */}
          {revokeError && (
            <div
              className="mb-3 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm flex items-start gap-2"
              data-testid="revoke-error-message"
              role="alert"
            >
              <span>⚠️ {revokeError}</span>
              <button onClick={() => setRevokeError('')} className="ml-auto text-red-400 hover:text-red-600 font-bold" aria-label="Dismiss error">×</button>
            </div>
          )}

          {activeConsents.length === 0 ? (
            <p className="text-gray-500 text-sm">No active provider access grants.</p>
          ) : (
            <div className="space-y-3">
              {activeConsents.map(consent => (
                <div
                  key={consent.id}
                  className="border border-green-200 bg-green-50 rounded-lg p-4"
                  data-testid={`active-consent-${consent.id}`}
                  data-consent-id={consent.id}
                  data-granted-at={consent.granted_at}
                  data-expires-at={consent.expires_at || ''}
                  data-scope={consent.scope}
                >
                  <div className="flex items-start justify-between">
                    <div>
                      <p className="font-medium text-gray-900">
                        {consent.provider_name || consent.provider_email || `Provider #${consent.provider_id}`}
                      </p>
                      <div className="mt-1 space-y-1">
                        <p className="text-sm text-gray-600">
                          <span className="font-medium">Scope:</span>{' '}
                          {SCOPE_OPTIONS.find(s => s.value === consent.scope)?.label || consent.scope}
                        </p>
                        <p className="text-sm text-gray-600">
                          <span className="font-medium">Purpose:</span>{' '}
                          {PURPOSE_OPTIONS.find(p => p.value === consent.purpose)?.label || consent.purpose}
                        </p>
                        <p className="text-sm text-gray-500" data-testid={`consent-${consent.id}-granted-timestamp`}>
                          Granted: {formatLocalDateTime(consent.granted_at)}
                          {' · '}
                          <span data-testid={`consent-${consent.id}-expiry`}>{formatExpiry(consent)}</span>
                        </p>
                        {consent.expires_at && (
                          <p className="text-xs text-gray-400" data-testid={`consent-${consent.id}-expires-at-iso`}>
                            Expires: <span data-testid={`consent-${consent.id}-expires-value`}>{new Date(consent.expires_at).toISOString()}</span>
                          </p>
                        )}
                        {/* Feature #366: Remaining time countdown */}
                        {consent.expires_at && (() => {
                          const remaining = formatRemainingTime(consent.expires_at);
                          if (!remaining) return null;
                          const isExpired = remaining === 'Expired';
                          const diffMs = new Date(consent.expires_at) - now;
                          const remainingSecs = Math.max(0, Math.floor(diffMs / 1000));
                          return (
                            <p
                              className={`text-xs font-medium ${isExpired ? 'text-red-500' : 'text-amber-600'}`}
                              data-testid={`consent-${consent.id}-remaining-time`}
                              data-remaining-seconds={remainingSecs}
                            >
                              ⏱ {remaining}
                            </p>
                          );
                        })()}
                        {consent.npi && (
                          <p className="text-xs text-gray-400">NPI: {consent.npi} · Facility: {consent.facility}</p>
                        )}
                      </div>
                    </div>
                    <button
                      onClick={() => handleRevoke(consent.id)}
                      disabled={revokingIds.has(consent.id)}
                      className={`text-sm font-medium ${revokingIds.has(consent.id) ? 'text-gray-400 cursor-not-allowed' : 'text-red-500 hover:text-red-700'}`}
                      data-testid={`revoke-${consent.id}`}
                    >
                      {revokingIds.has(consent.id) ? 'Revoking...' : 'Revoke'}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* History */}
        {inactiveConsents.length > 0 && (
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              Access History ({inactiveConsents.length})
            </h2>
            <div className="space-y-3">
              {inactiveConsents.map(consent => (
                <div key={consent.id} className="border border-gray-200 bg-gray-50 rounded-lg p-4 opacity-70">
                  <p className="font-medium text-gray-700">
                    {consent.provider_name || consent.provider_email || `Provider #${consent.provider_id}`}
                  </p>
                  <div className="mt-1">
                    <p className="text-sm text-gray-500" data-testid="consent-history-timestamp">
                      Scope: {SCOPE_OPTIONS.find(s => s.value === consent.scope)?.label || consent.scope}
                      {' · '}
                      {formatExpiry(consent)}
                      {' · '}
                      Granted: {formatLocalDateTime(consent.granted_at)}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
        {/* Feature #134: Full Medical Sovereignty Section */}
        <div className="bg-white rounded-lg shadow p-6 mt-6" data-testid="sovereignty-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-2 flex items-center gap-2">
            <span>⚖️</span> Medical Sovereignty Features
          </h2>
          <p className="text-sm text-gray-600 mb-4">
            Advanced consent and custody controls unlock as your 0dentity score increases. Your current score:{' '}
            <strong className={odentityScore !== null && odentityScore >= 75 ? 'text-green-600' : 'text-amber-600'}>
              {odentityScore !== null ? odentityScore.toFixed(1) : '—'}
            </strong>
          </p>

          <div className="space-y-3">
            {/* Score-gated features */}
            {[
              {
                feature: 'provider_sharing',
                label: 'Provider Record Sharing',
                description: 'Share your health records with verified providers via consent events.',
                score_minimum: 25,
                icon: '🏥',
              },
              {
                feature: 'pace_trustee_appointment',
                label: 'PACE Trustee Appointment',
                description: 'Appoint trusted people for recovery and emergency coordination.',
                score_minimum: 40,
                icon: '🤝',
              },
              {
                feature: 'advance_directive_binding',
                label: 'Advance Directive Binding',
                description: 'Bind legal healthcare directives to your health record.',
                score_minimum: 60,
                icon: '📋',
              },
              {
                feature: 'full_medical_sovereignty',
                label: 'Full Medical Sovereignty',
                description: 'Complete control over your health data: export, portability, and consent-bound access.',
                score_minimum: 75,
                icon: '👑',
              },
              {
                feature: 'verified_identity_export',
                label: 'Verified Identity Export',
                description: 'Export your fully verified W3C DID-compliant health identity to external networks.',
                score_minimum: 90,
                icon: '🌐',
              },
            ].map((gate) => {
              const currentScore = odentityScore !== null ? odentityScore : 0;
              const unlocked = currentScore >= gate.score_minimum;
              return (
                <div
                  key={gate.feature}
                  className={`flex items-start gap-3 p-4 rounded-lg border ${
                    unlocked ? 'border-green-200 bg-green-50' : 'border-gray-200 bg-gray-50'
                  }`}
                  data-testid={`sovereignty-gate-${gate.feature}`}
                >
                  <span className="text-xl mt-0.5">{gate.icon}</span>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-0.5">
                      <span className={`font-medium text-sm ${unlocked ? 'text-green-800' : 'text-gray-700'}`}>
                        {gate.label}
                      </span>
                      {unlocked ? (
                        <span className="text-xs bg-green-100 text-green-700 px-2 py-0.5 rounded-full font-medium" data-testid={`unlocked-${gate.feature}`}>
                          ✓ Unlocked
                        </span>
                      ) : (
                        <span className="text-xs bg-gray-200 text-gray-600 px-2 py-0.5 rounded-full font-medium" data-testid={`locked-${gate.feature}`}>
                          🔒 Requires {gate.score_minimum}+
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-gray-500">{gate.description}</p>
                    {!unlocked && (
                      <div className="mt-2 flex items-center gap-2">
                        <div className="flex-1 bg-gray-200 rounded-full h-1.5">
                          <div
                            className="bg-sky-400 h-1.5 rounded-full transition-all"
                            style={{ width: `${Math.min(100, (currentScore / gate.score_minimum) * 100)}%` }}
                          />
                        </div>
                        <span className="text-xs text-gray-400 font-mono">{currentScore.toFixed(0)}/{gate.score_minimum}</span>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          {odentityScore !== null && odentityScore < 75 && (
            <div className="mt-4 p-3 bg-sky-50 border border-sky-200 rounded-lg text-sm text-sky-700" data-testid="sovereignty-locked-explanation">
              <strong>Full Medical Sovereignty requires a score of 75+.</strong>{' '}
              Complete your profile across all 6 identity dimensions to unlock autonomous health data ownership.
              <button
                className="ml-2 underline font-medium hover:text-sky-900"
                onClick={() => navigate('/odentity')}
              >
                Improve your score →
              </button>
            </div>
          )}
          {odentityScore !== null && odentityScore >= 75 && (
            <div className="mt-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-700" data-testid="sovereignty-unlocked-msg">
              <strong>✓ Full Medical Sovereignty unlocked!</strong>{' '}
              You have full autonomous control over your health data. Advanced consent and custody controls are active.
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
