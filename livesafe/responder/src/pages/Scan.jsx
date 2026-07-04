import React, { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate, useSearchParams } from 'react-router-dom';
import api from '../services/api';

function Scan() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [scanning, setScanning] = useState(false);
  const [subscriberInput, setSubscriberInput] = useState('');
  const [scanRecord, setScanRecord] = useState(null);
  const [patientData, setPatientData] = useState(null);
  const [error, setError] = useState('');
  const [location, setLocation] = useState('');

  // Expanded access request state
  const [expandedAccessStatus, setExpandedAccessStatus] = useState(null); // null | 'requesting' | 'pending' | 'approved' | 'error'
  const [expandedAccessMessage, setExpandedAccessMessage] = useState('');
  const [expandedWorkflow, setExpandedWorkflow] = useState(null);

  // Flag for follow-up state (Feature #76)
  const [flagMode, setFlagMode] = useState(false); // show/hide notes textarea
  const [flagNotes, setFlagNotes] = useState('');
  const [flagStatus, setFlagStatus] = useState(null); // null | 'saving' | 'saved' | 'error'
  const [flagMessage, setFlagMessage] = useState('');
  const [isFlagged, setIsFlagged] = useState(false);

  // State for invalid QR code detection
  const [isInvalidQr, setIsInvalidQr] = useState(false);

  // Handle QR deep-link URL params on mount: ?did=xxx&token=yyy
  // This enables QR codes that encode the responder portal URL to auto-initiate scans
  useEffect(() => {
    const didParam = searchParams.get('did');
    if (didParam && didParam.startsWith('did:exo:')) {
      // Pre-fill the subscriber input with the DID from QR scan URL
      setSubscriberInput(didParam);
      // Auto-initiate scan after a brief delay to allow component to settle
      setTimeout(() => {
        handleScanWithDid(didParam);
      }, 500);
    }
  }, []);

  // Parse subscriber DID from raw input, QR payload JSON, or QR scan URL
  // Returns { did, valid, reason }
  const resolveSubscriberDid = (input) => {
    const trimmed = input.trim();

    // Check if it's a LiveSafe QR scan URL (from responder portal deep-link)
    // Format: http://localhost:3002/scan?did=did:exo:subscriber:xxx&token=yyy
    if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
      try {
        const url = new URL(trimmed);
        const didInUrl = url.searchParams.get('did');
        if (didInUrl && didInUrl.startsWith('did:exo:')) {
          return { did: didInUrl, valid: true };
        }
        // URL present but not a LiveSafe scan URL
        return {
          did: null,
          valid: false,
          reason: `This QR code links to a website (${trimmed.substring(0, 60)}${trimmed.length > 60 ? '...' : ''}) — not a LiveSafe emergency card. Please scan a patient's LiveSafe QR card.`,
        };
      } catch (_) {
        return {
          did: null,
          valid: false,
          reason: 'Invalid URL format in QR code.',
        };
      }
    }

    // Check if it's JSON QR payload (legacy format)
    try {
      const parsed = JSON.parse(trimmed);
      if (parsed.did) {
        // Validate the DID looks like a LiveSafe DID
        if (parsed.did.startsWith('did:exo:')) {
          return { did: parsed.did, valid: true };
        }
        return {
          did: null,
          valid: false,
          reason: `This QR code contains an unrecognized identity format. Expected a LiveSafe DID (did:exo:...) but got: "${parsed.did.substring(0, 40)}..."`,
        };
      }
      // JSON but no 'did' field — not a LiveSafe QR
      return {
        did: null,
        valid: false,
        reason: 'This QR code is not a LiveSafe emergency card. It appears to be a generic QR code without medical identity data.',
      };
    } catch (_) {}

    // Check if it's a raw DID
    if (trimmed.startsWith('did:exo:')) {
      return { did: trimmed, valid: true };
    }

    // Any other non-LiveSafe input
    if (trimmed.length > 0 && !trimmed.startsWith('did:')) {
      return {
        did: null,
        valid: false,
        reason: 'This does not appear to be a LiveSafe emergency card QR code. Please scan a valid LiveSafe card QR code or enter a patient DID (did:exo:subscriber:...).',
      };
    }

    // Generic DID (not did:exo: prefix) — might be an unsupported system
    return {
      did: trimmed,
      valid: trimmed.startsWith('did:exo:'),
      reason: trimmed.startsWith('did:exo:') ? null : 'Unrecognized DID format. LiveSafe uses did:exo:subscriber:... identifiers.',
    };
  };

  // Core scan execution — accepts a validated subscriber DID directly
  const executeScan = async (subscriberDid) => {
    setScanning(true);
    setError('');
    setIsInvalidQr(false);
    setScanRecord(null);
    setPatientData(null);
    try {
      // Step 1: Create scan record (emergency consent applied)
      const scanRes = await api.post('/scan', {
        subscriber_did: subscriberDid,
        responder_id: user?.id,
        location: location || 'Field',
        scan_type: 'emergency',
      });
      setScanRecord(scanRes.data);

      // Step 2: Fetch critical patient health data
      const dataRes = await api.get('/scan/data/' + encodeURIComponent(subscriberDid));
      setPatientData(dataRes.data);
    } catch (err) {
      if (err.response?.status === 404) {
        setError('Subscriber not found. This LiveSafe QR code may belong to an unregistered patient. Verify the QR code is correct.');
      } else {
        setError(err.response?.data?.error || 'Scan failed. Please try again.');
      }
    } finally {
      setScanning(false);
    }
  };

  // Handle scan from URL query param (QR deep-link)
  const handleScanWithDid = async (did) => {
    if (!did || !did.startsWith('did:exo:')) return;
    await executeScan(did);
  };

  const handleScan = async () => {
    if (!subscriberInput.trim()) {
      setError('Enter a subscriber DID or paste QR code data');
      return;
    }

    setScanning(true);
    setError('');
    setIsInvalidQr(false);
    setScanRecord(null);
    setPatientData(null);

    const resolved = resolveSubscriberDid(subscriberInput);

    // Reject non-LiveSafe QR codes before making any API call
    if (!resolved.valid) {
      setError(resolved.reason || 'Invalid QR code: Not a LiveSafe emergency card.');
      setIsInvalidQr(true);
      setScanning(false);
      return;
    }

    const subscriberDid = resolved.did;
    setScanning(false); // executeScan will re-set scanning=true
    await executeScan(subscriberDid);
  };

  // Feature #286: Reset portal to scan-ready state (clears ALL scan state including flags/expanded access)
  const handleClearScan = () => {
    setSubscriberInput('');
    setError('');
    setIsInvalidQr(false);
    setScanRecord(null);
    setPatientData(null);
    setLocation('');
    // Reset flag state
    setFlagMode(false);
    setFlagNotes('');
    setFlagStatus(null);
    setFlagMessage('');
    setIsFlagged(false);
    // Reset expanded access state
    setExpandedAccessStatus(null);
    setExpandedAccessMessage('');
    setExpandedWorkflow(null);
  };

  const handleRequestExpandedAccess = async () => {
    if (!scanRecord?.id) return;
    setExpandedAccessStatus('requesting');
    setExpandedAccessMessage('');
    try {
      const res = await api.post(`/scan/${scanRecord.id}/request-expanded-access`);
      setExpandedWorkflow(res.data.workflow);
      setExpandedAccessStatus('pending');
      setExpandedAccessMessage(res.data.message || 'Expanded access request submitted. Awaiting trustee approval.');
    } catch (err) {
      setExpandedAccessStatus('error');
      setExpandedAccessMessage(err.response?.data?.error || 'Failed to submit expanded access request.');
    }
  };

  const handleFlagForFollowup = async () => {
    if (!scanRecord?.id) return;
    // Feature #273: Idempotency guard - prevent rapid double submissions
    if (flagStatus === 'saving') return;
    if (isFlagged) return; // Already flagged - nothing to do
    setFlagStatus('saving');
    setFlagMessage('');
    try {
      const res = await api.patch(`/scan/${scanRecord.id}/flag`, {
        flagged: true,
        notes: flagNotes || null,
      });
      setIsFlagged(true);
      setFlagStatus('saved');
      setFlagMessage(res.data.message || 'Scan flagged for follow-up');
      setFlagMode(false);
    } catch (err) {
      setFlagStatus('error');
      setFlagMessage(err.response?.data?.error || 'Failed to flag scan');
    }
  };

  const formatDnr = (status) => {
    if (!status) return 'Not specified';
    const labels = {
      none: 'No DNR/Advance Directive',
      dnr: 'DNR - Do Not Resuscitate',
      polst: 'POLST on file',
      living_will: 'Living Will',
      full_code: 'Full Code',
    };
    return labels[status] || status;
  };

  const formatDate = (dateStr) => {
    if (!dateStr) return 'Not on file';
    try { return new Date(dateStr).toLocaleDateString(); } catch (_) { return dateStr; }
  };

  return (
    <div className="min-h-screen bg-gray-900 text-white" style={{ fontSize: '18px' }} data-testid="high-contrast-responder" data-high-contrast="true">
      <nav className="bg-red-700 text-white shadow-lg">
        <div className="max-w-4xl mx-auto px-4 py-3 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <h1 className="text-xl font-bold">LiveSafe<span className="text-amber-300">.ai</span></h1>
            <span className="text-sm bg-red-800 px-2 py-1 rounded font-semibold">SCAN MODE</span>
          </div>
          <button
            onClick={() => navigate('/dashboard')}
            className="text-base bg-red-800 hover:bg-red-900 px-4 py-2 rounded transition font-semibold"
            style={{ minHeight: '48px', minWidth: '48px' }}
          >
            ← Back
          </button>
        </div>
      </nav>

      <div className="max-w-2xl mx-auto px-4 py-6">
        <div
          id="scan-interface"
          data-testid="scan-interface"
          className="bg-gray-800 rounded-2xl border-2 border-red-500 p-8 mb-6 text-center"
        >
          <div className="mb-6">
            <div
              id="scan-area"
              className="w-48 h-48 mx-auto border-4 border-dashed border-red-400 rounded-2xl flex items-center justify-center mb-4 bg-gray-700"
              style={{ minWidth: '192px', minHeight: '192px' }}
            >
              <div className="text-center">
                <svg className="w-16 h-16 mx-auto text-red-400 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                    d="M12 4v1m6 11h2m-6 0h-2v4m0-11v3m0 0h.01M12 12h4.01M16 20h4M4 12h4m12 0h.01M5 8h2a1 1 0 001-1V5a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1zm12 0h2a1 1 0 001-1V5a1 1 0 00-1-1h-2a1 1 0 00-1 1v2a1 1 0 001 1zM5 20h2a1 1 0 001-1v-2a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1z"
                  />
                </svg>
                <p className="text-red-300 text-lg font-semibold">QR Scan Area</p>
              </div>
            </div>
            <h2 className="text-2xl font-bold text-white mb-2">Emergency Patient Scan</h2>
            <p className="text-gray-300 text-lg">Scan a patient's LiveSafe QR card or enter their DID below</p>
          </div>

          <div className="space-y-4 text-left">
            <div>
              <label className="block text-gray-300 text-lg font-semibold mb-2">
                Subscriber DID or QR Payload
              </label>
              <textarea
                id="subscriber-id-input"
                data-testid="subscriber-id-input"
                value={subscriberInput}
                onChange={(e) => setSubscriberInput(e.target.value)}
                placeholder={'did:exo:subscriber:...\n\nor paste QR JSON: {"did":"did:exo:subscriber:...","emergency_token":"..."}'}
                rows={3}
                className="w-full px-4 py-4 bg-gray-700 border-2 border-gray-600 rounded-xl text-white text-base placeholder-gray-500 focus:border-red-400 focus:outline-none resize-none"
                style={{ minHeight: '80px', fontSize: '16px', fontFamily: 'monospace' }}
              />
            </div>

            <div>
              <label className="block text-gray-300 text-lg font-semibold mb-2">
                Location (optional)
              </label>
              <input
                type="text"
                id="location-input"
                data-testid="location-input"
                value={location}
                onChange={(e) => setLocation(e.target.value)}
                placeholder="e.g., Field, Highway 101, Building 3"
                className="w-full px-4 py-4 bg-gray-700 border-2 border-gray-600 rounded-xl text-white text-lg placeholder-gray-400 focus:border-red-400 focus:outline-none"
                style={{ minHeight: '56px', fontSize: '18px' }}
              />
            </div>
          </div>
        </div>

        <button
          id="scan-button"
          data-testid="scan-button"
          onClick={handleScan}
          disabled={scanning}
          className={`w-full py-6 rounded-2xl text-2xl font-bold transition shadow-lg ${
            scanning ? 'bg-gray-600 text-gray-400 cursor-wait' : 'bg-red-800 hover:bg-red-700 text-white active:bg-red-900'
          }`}
          style={{ minHeight: '72px', fontSize: '24px' }}
        >
          {scanning ? (
            <span className="flex items-center justify-center space-x-3">
              <svg className="animate-spin h-8 w-8" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              <span>Scanning...</span>
            </span>
          ) : '🔍 SCAN PATIENT'}
        </button>

        {/* Feature #299: Dedicated loading indicator between scan initiation and data display */}
        {scanning && (
          <div
            className="mt-4 rounded-xl p-6 bg-gray-800 border-2 border-red-500 text-center"
            data-testid="scan-loading"
            role="status"
            aria-live="polite"
            aria-label="Resolving patient identity"
          >
            <div className="flex flex-col items-center gap-4">
              <div className="flex items-center justify-center gap-3">
                <svg className="animate-spin h-10 w-10 text-red-400" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                </svg>
                <span className="text-red-300 text-xl font-bold" data-testid="scan-loading-text">Resolving patient identity...</span>
              </div>
              <p className="text-gray-400 text-base">Applying emergency consent and fetching critical health data</p>
              <div className="flex gap-2 mt-1">
                <span className="w-2 h-2 bg-red-400 rounded-full animate-bounce" style={{ animationDelay: '0ms' }}></span>
                <span className="w-2 h-2 bg-red-400 rounded-full animate-bounce" style={{ animationDelay: '150ms' }}></span>
                <span className="w-2 h-2 bg-red-400 rounded-full animate-bounce" style={{ animationDelay: '300ms' }}></span>
              </div>
            </div>
          </div>
        )}

        {error && (
          <div
            className={`mt-4 rounded-xl p-4 text-center ${isInvalidQr ? 'bg-amber-900 border-2 border-amber-500' : 'bg-red-900 border-2 border-red-500'}`}
            data-testid={isInvalidQr ? 'invalid-qr-error' : 'scan-error'}
          >
            {isInvalidQr && (
              <p className="text-amber-300 text-xl font-bold mb-2">⚠️ Invalid QR Code</p>
            )}
            <p className={`${isInvalidQr ? 'text-amber-200' : 'text-red-200'} text-lg font-semibold`}>{error}</p>
            {isInvalidQr && (
              <div className="mt-3">
                <p className="text-amber-400 text-sm mb-3">
                  Please scan a valid <strong>LiveSafe Emergency Card</strong> QR code.
                </p>
                <button
                  onClick={handleClearScan}
                  className="px-6 py-3 bg-amber-600 hover:bg-amber-500 text-white rounded-xl font-bold text-base transition"
                  data-testid="clear-scan-btn"
                >
                  🔄 Clear & Scan Again
                </button>
              </div>
            )}
          </div>
        )}

        {patientData && (
          <div className="mt-6 space-y-4" data-testid="patient-data">
            {/* Feature #286: New Scan button — allows responder to dismiss results and return to scan-ready state */}
            <div className="flex justify-end">
              <button
                id="new-scan-btn"
                data-testid="new-scan-btn"
                onClick={handleClearScan}
                className="px-6 py-3 bg-gray-700 hover:bg-gray-600 border-2 border-gray-500 text-white rounded-xl font-bold text-base transition flex items-center gap-2"
                style={{ minHeight: '48px' }}
                aria-label="Clear scan results and start new scan"
              >
                🔄 New Scan
              </button>
            </div>

            {/* Scan Success Banner */}
            <div className="bg-green-900 border-2 border-green-500 rounded-2xl p-4" data-testid="scan-success">
              <div className="flex items-center gap-3 mb-2">
                <span className="text-2xl">✅</span>
                <div>
                  <h3 className="text-xl font-bold text-green-300">DID Resolved — Emergency Access Granted</h3>
                  <p className="text-green-400 text-sm">
                    Responder verified • Emergency consent applied • Scan #{scanRecord?.id}
                  </p>
                </div>
              </div>
              <div className="text-sm text-green-300 font-mono break-all" data-testid="resolved-did">
                {patientData.subscriber?.did}
              </div>
              {scanRecord?.pace_alerts_sent > 0 && (
                <div className="mt-3 bg-green-800 rounded-xl p-3" data-testid="pace-alerts-dispatched">
                  <p className="text-green-200 text-sm font-semibold">
                    🔔 PACE Alert Dispatched — {scanRecord.pace_alerts_sent} trustee{scanRecord.pace_alerts_sent !== 1 ? 's' : ''} notified
                  </p>
                  <p className="text-green-400 text-xs mt-1">
                    All PACE trustees have been alerted about this emergency scan
                  </p>
                </div>
              )}
              {/* Feature #275: Show deduplication indicator when duplicate alerts were suppressed */}
              {scanRecord?.pace_alerts_deduplicated > 0 && scanRecord?.pace_alerts_sent === 0 && (
                <div className="mt-3 bg-blue-900 rounded-xl p-3" data-testid="pace-alerts-deduplicated">
                  <p className="text-blue-200 text-sm font-semibold">
                    🔕 PACE Alerts Suppressed — {scanRecord.pace_alerts_deduplicated} duplicate{scanRecord.pace_alerts_deduplicated !== 1 ? 's' : ''} skipped
                  </p>
                  <p className="text-blue-400 text-xs mt-1">
                    Trustees were already alerted within the last {scanRecord.pace_alert_dedup_window_minutes || 5} minutes — no duplicate notifications sent
                  </p>
                </div>
              )}
            </div>

            {/* Flag for Follow-up — Feature #76 */}
            <div className="bg-gray-800 border-2 border-yellow-600 rounded-2xl p-6" data-testid="flag-followup-section">
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-xl font-bold text-yellow-300">🚩 Flag for Follow-up</h3>
                {isFlagged && (
                  <span
                    className="px-3 py-1 bg-yellow-800 text-yellow-200 rounded-full text-sm font-bold border border-yellow-600"
                    data-testid="flagged-badge"
                  >
                    ✅ Flagged
                  </span>
                )}
              </div>
              <p className="text-gray-300 text-base mb-4">
                Mark this scan for debrief or outcome tracking. Agency admin will see flagged scans separately.
              </p>

              {!isFlagged && !flagMode && (
                <button
                  id="flag-followup-btn"
                  data-testid="flag-followup-btn"
                  onClick={() => setFlagMode(true)}
                  className="w-full py-4 bg-yellow-700 hover:bg-yellow-600 text-white rounded-xl text-lg font-bold transition"
                  style={{ minHeight: '56px' }}
                >
                  🚩 Flag for Follow-up
                </button>
              )}

              {flagMode && !isFlagged && (
                <div className="space-y-3" data-testid="flag-notes-form">
                  <div>
                    <label className="block text-gray-300 text-base font-semibold mb-2">
                      Follow-up Notes (optional)
                    </label>
                    <textarea
                      id="flag-notes-input"
                      data-testid="flag-notes-input"
                      value={flagNotes}
                      onChange={(e) => setFlagNotes(e.target.value)}
                      placeholder="Describe the reason for follow-up, outcome needed, etc."
                      rows={3}
                      className="w-full px-4 py-3 bg-gray-700 border-2 border-yellow-600 rounded-xl text-white text-base placeholder-gray-500 focus:border-yellow-400 focus:outline-none resize-none"
                      style={{ minHeight: '80px' }}
                    />
                  </div>
                  <div className="flex gap-3">
                    <button
                      data-testid="confirm-flag-btn"
                      onClick={handleFlagForFollowup}
                      disabled={flagStatus === 'saving'}
                      className="flex-1 py-4 bg-yellow-700 hover:bg-yellow-600 disabled:bg-gray-600 text-white rounded-xl text-lg font-bold transition"
                      style={{ minHeight: '56px' }}
                    >
                      {flagStatus === 'saving' ? '⏳ Saving...' : '✅ Confirm Flag'}
                    </button>
                    <button
                      data-testid="cancel-flag-btn"
                      onClick={() => { setFlagMode(false); setFlagStatus(null); setFlagMessage(''); }}
                      className="px-6 py-4 bg-gray-600 hover:bg-gray-500 text-white rounded-xl text-lg font-bold transition"
                      style={{ minHeight: '56px' }}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              )}

              {flagStatus === 'saved' && (
                <div
                  className="mt-3 bg-green-900 border-2 border-green-500 rounded-xl p-4"
                  data-testid="flag-success"
                >
                  <p className="text-green-200 text-base font-bold">✅ {flagMessage}</p>
                  {flagNotes && <p className="text-green-400 text-sm mt-1">Notes: {flagNotes}</p>}
                </div>
              )}

              {flagStatus === 'error' && (
                <div className="mt-3 bg-red-900 border border-red-500 rounded-xl p-4" data-testid="flag-error">
                  <p className="text-red-200 font-semibold">❌ {flagMessage}</p>
                </div>
              )}
            </div>

            {/* Request Expanded Access */}
            <div className="bg-gray-800 border-2 border-orange-500 rounded-2xl p-6" data-testid="expanded-access-section">
              <h3 className="text-xl font-bold text-orange-300 mb-3">🔓 Expanded Access</h3>
              <p className="text-gray-300 text-base mb-4">
                Emergency access shows critical info only. Request expanded access to full medical records — requires 2-of-4 trustee approvals.
              </p>

              {expandedAccessStatus === null && (
                <button
                  id="request-expanded-access-btn"
                  data-testid="request-expanded-access"
                  onClick={handleRequestExpandedAccess}
                  className="w-full py-4 bg-orange-600 hover:bg-orange-500 text-white rounded-xl text-lg font-bold transition"
                  style={{ minHeight: '56px' }}
                >
                  📋 Request Expanded Access
                </button>
              )}

              {expandedAccessStatus === 'requesting' && (
                <div className="text-center py-4">
                  <div className="animate-spin h-8 w-8 border-4 border-orange-400 border-t-transparent rounded-full mx-auto mb-2"></div>
                  <p className="text-orange-300 font-semibold">Submitting request...</p>
                </div>
              )}

              {expandedAccessStatus === 'pending' && (
                <div className="space-y-3" data-testid="expanded-access-pending">
                  <div className="flex items-center gap-3 bg-orange-900 rounded-xl p-4">
                    <span className="text-2xl">⏳</span>
                    <div>
                      <p className="text-orange-200 font-bold text-base">Expanded Access Request Pending</p>
                      <p className="text-orange-400 text-sm">Status: Awaiting Trustee Approval</p>
                    </div>
                  </div>
                  <p className="text-gray-300 text-sm">{expandedAccessMessage}</p>
                  {expandedWorkflow && (
                    <div className="bg-gray-700 rounded-xl p-3 text-sm">
                      <p className="text-gray-400">Workflow ID: <span className="text-white font-mono">#{expandedWorkflow.id}</span></p>
                      <p className="text-gray-400">Approvals needed: <span className="text-orange-300">{expandedWorkflow.required_signers - (expandedWorkflow.current_signers || 0)} more</span></p>
                      {expandedWorkflow.deadline_at && (
                        <p className="text-gray-400">Deadline: <span className="text-white">{new Date(expandedWorkflow.deadline_at).toLocaleTimeString()}</span></p>
                      )}
                    </div>
                  )}
                </div>
              )}

              {expandedAccessStatus === 'error' && (
                <div className="bg-red-900 border border-red-500 rounded-xl p-4" data-testid="expanded-access-error">
                  <p className="text-red-200 font-semibold">❌ {expandedAccessMessage}</p>
                  <button
                    onClick={() => setExpandedAccessStatus(null)}
                    className="mt-2 text-red-300 hover:text-red-100 text-sm underline"
                  >Try again</button>
                </div>
              )}
            </div>

            {/* Critical Info */}
            <div className="bg-gray-800 border-2 border-amber-500 rounded-2xl p-6" data-testid="critical-info">
              <h3 className="text-xl font-bold text-amber-300 mb-4">🏥 Critical Patient Info</h3>
              <div className="grid grid-cols-2 gap-4 text-lg">
                <div>
                  <p className="text-gray-400 text-sm font-semibold uppercase tracking-wide">Full Name</p>
                  <p className="text-white font-bold" data-testid="patient-name">
                    {[patientData.subscriber?.first_name, patientData.subscriber?.last_name].filter(Boolean).join(' ') || 'Not on file'}
                  </p>
                </div>
                <div>
                  <p className="text-gray-400 text-sm font-semibold uppercase tracking-wide">Date of Birth</p>
                  <p className="text-white font-bold" data-testid="patient-dob">
                    {formatDate(patientData.subscriber?.date_of_birth)}
                  </p>
                </div>
                <div>
                  <p className="text-gray-400 text-sm font-semibold uppercase tracking-wide">Blood Type</p>
                  <p className={`font-bold text-xl ${patientData.subscriber?.blood_type ? 'text-red-300' : 'text-gray-500'}`}
                     data-testid="patient-blood-type">
                    {patientData.subscriber?.blood_type || 'Not on file'}
                  </p>
                </div>
                <div>
                  <p className="text-gray-400 text-sm font-semibold uppercase tracking-wide">Organ Donor</p>
                  <p className="text-white font-bold" data-testid="patient-organ-donor">
                    {patientData.subscriber?.organ_donor ? '✅ Yes' : '❌ No'}
                  </p>
                </div>
              </div>

              <div className={`mt-4 p-4 rounded-xl border-2 ${
                patientData.subscriber?.dnr_status && patientData.subscriber.dnr_status !== 'none' && patientData.subscriber.dnr_status !== 'full_code'
                  ? 'bg-red-900 border-red-500' : 'bg-gray-700 border-gray-600'
              }`} data-testid="patient-dnr">
                <p className="text-gray-400 text-sm font-semibold uppercase tracking-wide mb-1">DNR / Advance Directive</p>
                <p className={`font-bold text-lg ${
                  patientData.subscriber?.dnr_status === 'dnr' ? 'text-red-300' :
                  patientData.subscriber?.dnr_status === 'polst' ? 'text-orange-300' : 'text-white'
                }`}>
                  {formatDnr(patientData.subscriber?.dnr_status)}
                </p>
              </div>
            </div>

            {/* Allergies */}
            <div className="bg-gray-800 border-2 border-red-700 rounded-2xl p-6" data-testid="patient-allergies">
              <h3 className="text-xl font-bold text-red-300 mb-3">
                ⚠️ Allergies ({patientData.allergies?.length || 0})
              </h3>
              {patientData.allergies?.length > 0 ? (
                <ul className="space-y-2">
                  {patientData.allergies.map((a, i) => (
                    <li key={i} className="flex items-start gap-3">
                      <span className={`px-2 py-0.5 rounded text-sm font-bold ${
                        a.severity === 'life-threatening' ? 'bg-red-700 text-red-100' :
                        a.severity === 'severe' ? 'bg-orange-700 text-orange-100' : 'bg-yellow-700 text-yellow-100'
                      }`}>{a.severity || 'unknown'}</span>
                      <span className="text-white font-semibold text-lg">{a.allergy}</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-gray-400">No known allergies on file</p>
              )}
            </div>

            {/* Medications */}
            <div className="bg-gray-800 border-2 border-blue-700 rounded-2xl p-6" data-testid="patient-medications">
              <h3 className="text-xl font-bold text-blue-300 mb-3">
                💊 Medications ({patientData.medications?.length || 0})
              </h3>
              {patientData.medications?.length > 0 ? (
                <ul className="space-y-2">
                  {patientData.medications.map((m, i) => (
                    <li key={i} className="text-white">
                      <span className="font-bold">{m.medication}</span>
                      {m.dosage && <span className="text-gray-300"> — {m.dosage}</span>}
                      {m.frequency && <span className="text-gray-400"> ({m.frequency})</span>}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-gray-400">No medications on file</p>
              )}
            </div>

            {/* Conditions */}
            <div className="bg-gray-800 border-2 border-purple-700 rounded-2xl p-6" data-testid="patient-conditions">
              <h3 className="text-xl font-bold text-purple-300 mb-3">
                🩺 Medical Conditions ({patientData.conditions?.length || 0})
              </h3>
              {patientData.conditions?.length > 0 ? (
                <ul className="space-y-2">
                  {patientData.conditions.map((c, i) => (
                    <li key={i} className="text-white">
                      <span className="font-bold">{c.condition_name}</span>
                      {c.diagnosed_date && <span className="text-gray-400"> (diagnosed {formatDate(c.diagnosed_date)})</span>}
                      {c.notes && <p className="text-gray-300 text-sm mt-0.5">{c.notes}</p>}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-gray-400">No conditions on file</p>
              )}
            </div>

            {/* Emergency Contacts */}
            <div className="bg-gray-800 border-2 border-green-700 rounded-2xl p-6" data-testid="patient-emergency-contacts">
              <h3 className="text-xl font-bold text-green-300 mb-3">
                📞 Emergency Contacts ({patientData.emergency_contacts?.length || 0})
              </h3>
              {patientData.emergency_contacts?.length > 0 ? (
                <ul className="space-y-3">
                  {patientData.emergency_contacts.map((c, i) => (
                    <li key={i} className="text-white">
                      <p className="font-bold">{c.name} <span className="text-gray-400 font-normal text-base">({c.relationship})</span></p>
                      <p className="text-green-300 text-lg font-mono">{c.phone}</p>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-gray-400">No emergency contacts on file</p>
              )}
            </div>

            {/* Insurance Cards — Feature #118: Only shown when subscriber configured as emergency_visible */}
            <div
              className={`bg-gray-800 border-2 rounded-2xl p-6 ${
                patientData.insurance_visible_to_er ? 'border-sky-500' : 'border-gray-600'
              }`}
              data-testid="patient-insurance"
            >
              <h3 className="text-xl font-bold mb-3" style={{ color: patientData.insurance_visible_to_er ? '#7dd3fc' : '#6b7280' }}>
                🏥 Insurance ({patientData.insurance?.length || 0})
              </h3>
              {patientData.insurance_visible_to_er && patientData.insurance?.length > 0 ? (
                <ul className="space-y-3">
                  {patientData.insurance.map((ins, i) => (
                    <li key={i} className="bg-gray-700 rounded-xl p-4" data-testid={`insurance-card-${i}`}>
                      <p className="text-sky-300 font-bold text-lg">{ins.title || 'Insurance Card'}</p>
                      <div className="grid grid-cols-2 gap-2 mt-2 text-base">
                        {ins.carrier && (
                          <div>
                            <span className="text-gray-400 text-sm block">Carrier</span>
                            <span className="text-white font-semibold" data-testid="insurance-carrier">{ins.carrier}</span>
                          </div>
                        )}
                        {ins.member_id && (
                          <div>
                            <span className="text-gray-400 text-sm block">Member ID</span>
                            <span className="text-white font-semibold font-mono" data-testid="insurance-member-id">{ins.member_id}</span>
                          </div>
                        )}
                        {ins.group_number && (
                          <div>
                            <span className="text-gray-400 text-sm block">Group #</span>
                            <span className="text-white font-semibold font-mono" data-testid="insurance-group">{ins.group_number}</span>
                          </div>
                        )}
                        {ins.expiry_date && (
                          <div>
                            <span className="text-gray-400 text-sm block">Expires</span>
                            <span className="text-white font-semibold" data-testid="insurance-expiry">{formatDate(ins.expiry_date)}</span>
                          </div>
                        )}
                      </div>
                    </li>
                  ))}
                </ul>
              ) : (
                <div className="text-gray-400 text-base" data-testid="insurance-hidden-msg">
                  <p>🔒 Insurance info not shared for emergency access</p>
                  <p className="text-sm mt-1 text-gray-500">Subscriber has not enabled emergency responder visibility for their insurance card.</p>
                </div>
              )}
            </div>
          </div>
        )}

        <div className="mt-6 grid grid-cols-2 gap-4">
          <button
            onClick={() => navigate('/scan/history')}
            className="bg-gray-800 hover:bg-gray-700 border-2 border-amber-500 rounded-xl p-5 text-center transition"
            style={{ minHeight: '80px' }}
          >
            <span className="text-amber-400 text-lg font-bold block">📋 Scan History</span>
            <span className="text-gray-400 text-base">View past scans</span>
          </button>
          <button
            onClick={() => navigate('/dashboard')}
            className="bg-gray-800 hover:bg-gray-700 border-2 border-blue-500 rounded-xl p-5 text-center transition"
            style={{ minHeight: '80px' }}
          >
            <span className="text-blue-400 text-lg font-bold block">🏠 Dashboard</span>
            <span className="text-gray-400 text-base">Return home</span>
          </button>
        </div>

        <div className="mt-8 text-center text-gray-500 text-base">
          <p>Logged in as: <span className="text-gray-400 font-mono">{user?.did || user?.email}</span></p>
          <p>Agency: <span className="text-gray-400">{user?.agency_name || 'Not assigned'}</span></p>
        </div>
      </div>
    </div>
  );
}

export default Scan;
