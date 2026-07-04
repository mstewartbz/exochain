import React, { useState, useEffect, useRef } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate, Link } from 'react-router-dom';
import api from '../services/api';
import Navbar from '../components/Navbar';

const PACE_ROLES = [
  {
    key: 'primary',
    letter: 'P',
    name: 'Primary',
    description: 'First point of contact for identity recovery and medical decisions. Holds the primary recovery role for your LiveSafe DID.',
    color: 'sky',
  },
  {
    key: 'alternate',
    letter: 'A',
    name: 'Alternate',
    description: 'Backup trustee who steps in if the Primary is unavailable. Holds the alternate key shard for identity recovery.',
    color: 'emerald',
  },
  {
    key: 'contingent',
    letter: 'C',
    name: 'Contingent',
    description: 'Trusted fallback if the first two routes fail or the emergency needs another ready human.',
    color: 'amber',
  },
  {
    key: 'emergency',
    letter: 'E',
    name: 'Emergency',
    description: 'Activated during emergency scans. Notified immediately when your card is scanned by first responders. Holds the emergency key shard.',
    color: 'rose',
  },
];

// Feature #305: Dashboard tab definitions
const DASHBOARD_TABS = [
  { key: 'overview', label: 'Overview' },
  { key: 'pace', label: 'PACE' },
  { key: 'card', label: 'Card' },
  { key: 'vault', label: 'Health Vault' },
];

function Dashboard() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  // Feature #305: active tab state
  const [activeTab, setActiveTab] = useState('overview');
  const [trustees, setTrustees] = useState([]);
  const [vssCeremony, setVssCeremony] = useState(null);
  const [nominateRole, setNominateRole] = useState(null);
  const [trusteeEmails, setTrusteeEmails] = useState({});
  const [nominateError, setNominateError] = useState('');
  const [nominateSuccess, setNominateSuccess] = useState('');
  const [nominateSubmitting, setNominateSubmitting] = useState(false);
  const [loadingTrustees, setLoadingTrustees] = useState(false);
  // Replacement workflow state
  const [replaceMode, setReplaceMode] = useState(null); // trusteeId being replaced
  const [replaceEmail, setReplaceEmail] = useState('');
  const [replaceError, setReplaceError] = useState('');
  const [replaceSuccess, setReplaceSuccess] = useState('');
  const [replaceSubmitting, setReplaceSubmitting] = useState(false);
  const [governanceWorkflows, setGovernanceWorkflows] = useState([]);
  // Status indicators
  const [cardStatus, setCardStatus] = useState(null);
  const [odentityScore, setOdentityScore] = useState(null);
  const [recordCount, setRecordCount] = useState(null);
  // Feature #401: Vault completeness score (section-based)
  const [vaultCompletenessData, setVaultCompletenessData] = useState(null);

  // Feature #295: Real-time scan alert toast notification
  const [scanToast, setScanToast] = useState(null); // { title, body_parsed, notification_id }
  const knownNotificationIdsRef = useRef(null); // null = not yet initialized
  const scanPollTimerRef = useRef(null);

  // Feature #361: PACE alert toast for screen reader announcements
  const [paceAlertToast, setPaceAlertToast] = useState(null); // { title, body_parsed, notification_id }
  // Feature #361: Screen-reader announcement text (updated when new notifications arrive)
  const [srAnnouncement, setSrAnnouncement] = useState('');

  const fetchTrustees = async () => {
    if (!user?.did) return;
    try {
      setLoadingTrustees(true);
      const res = await api.get(`/pace/trustees/${user.did}`);
      // Handle both old (array) and new (object) response formats
      if (Array.isArray(res.data)) {
        setTrustees(res.data);
        setVssCeremony(null);
      } else {
        setTrustees(res.data.trustees || []);
        setVssCeremony(res.data.vss_ceremony || null);
      }
    } catch (err) {
      console.error('Failed to fetch trustees:', err);
    } finally {
      setLoadingTrustees(false);
    }
  };

  const fetchGovernanceWorkflows = async () => {
    if (!user?.did) return;
    try {
      const res = await api.get(`/pace/governance/subscriber/${user.did}`);
      setGovernanceWorkflows(res.data || []);
    } catch (err) {
      console.error('Failed to fetch governance workflows:', err);
    }
  };

  const fetchCardStatus = async () => {
    try {
      const res = await api.get('/card/me');
      setCardStatus(res.data);
    } catch (err) {
      setCardStatus(null);
    }
  };

  const fetchOdentityScore = async () => {
    try {
      const res = await api.get('/odentity/me/score');
      setOdentityScore(res.data);
    } catch (err) {
      setOdentityScore(null);
    }
  };

  const fetchRecordCount = async () => {
    try {
      const res = await api.get('/records');
      setRecordCount(Array.isArray(res.data) ? res.data.length : (res.data?.records?.length ?? 0));
    } catch (err) {
      setRecordCount(0);
    }
  };

  // Feature #401: Fetch section-based vault completeness score
  const fetchVaultCompleteness = async () => {
    try {
      const res = await api.get('/subscribers/vault-completeness');
      setVaultCompletenessData(res.data);
    } catch (err) {
      setVaultCompletenessData(null);
    }
  };

  useEffect(() => {
    fetchTrustees();
    fetchGovernanceWorkflows();
    fetchCardStatus();
    fetchOdentityScore();
    fetchRecordCount();
    fetchVaultCompleteness();
  }, [user?.did]);

  // Feature #295 / #361: Poll for new card_scan and pace_alert notifications to show real-time toast
  const checkForScanAlerts = async () => {
    if (!user?.did) return;
    try {
      const res = await api.get('/notifications');
      const notifications = res.data?.notifications || [];
      // Feature #295: card scan alerts
      const scanNotifs = notifications.filter(n => n.notification_type === 'card_scan' && !n.read);
      // Feature #361: PACE alert notifications
      const paceAlertNotifs = notifications.filter(n => n.notification_type === 'pace_alert' && !n.read);

      if (knownNotificationIdsRef.current === null) {
        // First poll: just record what exists already, don't toast for old ones
        const allAlertIds = [...scanNotifs, ...paceAlertNotifs].map(n => n.id);
        knownNotificationIdsRef.current = new Set(allAlertIds);
        return;
      }

      // Find new scan notifications not seen before
      const newScans = scanNotifs.filter(n => !knownNotificationIdsRef.current.has(n.id));
      if (newScans.length > 0) {
        const latest = newScans[0];
        let bodyParsed = {};
        try { bodyParsed = JSON.parse(latest.body || '{}'); } catch (e) { /* ignore */ }
        setScanToast({
          notification_id: latest.id,
          title: latest.title || 'Emergency card scanned',
          body_parsed: bodyParsed,
        });
        // Announce to screen readers (Feature #361)
        setSrAnnouncement(`Alert: ${latest.title || 'Emergency card scanned'}`);
        // Add new IDs to known set
        newScans.forEach(n => knownNotificationIdsRef.current.add(n.id));
      }

      // Feature #361: Find new PACE alert notifications not seen before
      const newPaceAlerts = paceAlertNotifs.filter(n => !knownNotificationIdsRef.current.has(n.id));
      if (newPaceAlerts.length > 0) {
        const latest = newPaceAlerts[0];
        let bodyParsed = {};
        try { bodyParsed = JSON.parse(latest.body || '{}'); } catch (e) { /* ignore */ }
        setPaceAlertToast({
          notification_id: latest.id,
          title: latest.title || 'PACE Alert',
          body_parsed: bodyParsed,
        });
        // Announce to screen readers (Feature #361)
        setSrAnnouncement(`PACE Alert: ${latest.title || 'New PACE notification'}. ${bodyParsed.subscriber_name ? 'Subscriber: ' + bodyParsed.subscriber_name + '.' : ''}`);
        // Add new IDs to known set
        newPaceAlerts.forEach(n => knownNotificationIdsRef.current.add(n.id));
      }
    } catch (err) {
      // Non-fatal: polling failure doesn't break dashboard
    }
  };

  useEffect(() => {
    if (!user?.did) return;
    // Start polling every 15 seconds for scan alert toasts (Feature #295)
    checkForScanAlerts(); // initial check
    scanPollTimerRef.current = setInterval(checkForScanAlerts, 15000);
    return () => {
      if (scanPollTimerRef.current) clearInterval(scanPollTimerRef.current);
    };
  }, [user?.did]);

  const dismissScanToast = () => {
    setScanToast(null);
  };

  // Feature #361: Dismiss PACE alert toast
  const dismissPaceAlertToast = () => {
    setPaceAlertToast(null);
  };

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const getTrusteeForRole = (roleKey) => {
    return trustees.find(t => t.role === roleKey);
  };

  const handleNominate = async (roleKey) => {
    const email = trusteeEmails[roleKey];
    if (!email) return;

    setNominateError('');
    setNominateSuccess('');

    // Client-side duplicate check
    const emailLower = email.toLowerCase().trim();
    const existingTrustee = trustees.find(t => t.email.toLowerCase().trim() === emailLower);
    if (existingTrustee) {
      setNominateError('Duplicate trustee email: ' + emailLower + ' is already nominated as ' + existingTrustee.role + '. The same person cannot fill multiple PACE roles.');
      return;
    }

    setNominateSubmitting(true);
    try {
      await api.post('/pace/trustees', {
        subscriber_id: user.id,
        trustees: [{ email: emailLower, role: roleKey }],
      });
      const roleName = PACE_ROLES.find(r => r.key === roleKey)?.name || roleKey;
      setNominateSuccess(`${roleName} trustee invitation sent to ${emailLower}!`);
      setTrusteeEmails(prev => ({ ...prev, [roleKey]: '' }));
      setNominateRole(null);
      fetchTrustees();
    } catch (err) {
      setNominateError(err.response?.data?.error || 'Failed to nominate trustee');
    } finally {
      setNominateSubmitting(false);
    }
  };

  const handleReplace = async (trusteeId) => {
    if (!replaceEmail) return;
    setReplaceError('');
    setReplaceSuccess('');
    setReplaceSubmitting(true);
    try {
      const res = await api.post(`/pace/trustees/${trusteeId}/replace`, {
        new_email: replaceEmail.toLowerCase().trim(),
        subscriber_did: user.did,
      });
      setReplaceSuccess(`Replacement workflow created (id: ${res.data.workflow_id}). Need 2 trustee approvals. Subscriber has already signed.`);
      setReplaceMode(null);
      setReplaceEmail('');
      fetchTrustees();
      fetchGovernanceWorkflows();
    } catch (err) {
      setReplaceError(err.response?.data?.error || 'Failed to initiate replacement');
    } finally {
      setReplaceSubmitting(false);
    }
  };

  const filledCount = PACE_ROLES.filter(r => getTrusteeForRole(r.key)).length;
  const acceptedCount = trustees.filter(t => t.status === 'accepted').length;
  const cardIsActive = cardStatus?.card?.status === 'active';
  const cardIssued = !!cardStatus?.card;
  const overallScore = odentityScore?.composite_score ?? odentityScore?.overall_score ?? odentityScore?.total_score ?? null;
  // Feature #401: Use section-based vault completeness (5 sections × 20% each: profile, allergies, medications, conditions, insurance_card)
  const vaultCompleteness = vaultCompletenessData !== null ? vaultCompletenessData.score : (recordCount !== null ? (recordCount > 0 ? Math.min(100, recordCount * 20) : 0) : null);
  const vaultSections = vaultCompletenessData?.sections || null;

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      {/* Feature #361: Screen-reader-only live region for notification announcements */}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
        data-testid="notification-announcement"
      >
        {srAnnouncement}
      </div>

      {/* Feature #295: Real-time scan alert toast notification */}
      {scanToast && (
        <div
          className="fixed top-4 right-4 z-50 max-w-sm w-full bg-sky-600 text-white rounded-xl shadow-lg p-4 border border-sky-500"
          data-testid="scan-alert-toast"
          role="alert"
          aria-live="assertive"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-start gap-3 flex-1 min-w-0">
              <span className="text-2xl flex-shrink-0">📱</span>
              <div className="flex-1 min-w-0">
                <p className="font-semibold text-sm" data-testid="scan-toast-title">{scanToast.title}</p>
                {scanToast.body_parsed.scan_time && (
                  <p className="text-xs text-sky-100 mt-0.5" data-testid="scan-toast-time">
                    Time: {new Date(scanToast.body_parsed.scan_time).toLocaleString()}
                  </p>
                )}
                {scanToast.body_parsed.responder_agency && (
                  <p className="text-xs text-sky-100 mt-0.5" data-testid="scan-toast-agency">
                    Agency: {scanToast.body_parsed.responder_agency}
                  </p>
                )}
                {scanToast.body_parsed.location && (
                  <p className="text-xs text-sky-100 mt-0.5" data-testid="scan-toast-location">
                    Location: {scanToast.body_parsed.location}
                  </p>
                )}
              </div>
            </div>
            <button
              onClick={dismissScanToast}
              className="text-sky-200 hover:text-white flex-shrink-0 text-lg leading-none font-bold"
              aria-label="Dismiss scan alert"
              data-testid="dismiss-scan-toast"
            >
              ×
            </button>
          </div>
        </div>
      )}

      {/* Feature #361: PACE alert toast notification for screen readers */}
      {paceAlertToast && (
        <div
          className="fixed top-4 left-4 z-50 max-w-sm w-full bg-red-600 text-white rounded-xl shadow-lg p-4 border border-red-500"
          data-testid="pace-alert-toast"
          role="alert"
          aria-live="assertive"
          aria-atomic="true"
          aria-label={`PACE Alert: ${paceAlertToast.title}`}
        >
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-start gap-3 flex-1 min-w-0">
              <span className="text-2xl flex-shrink-0" aria-hidden="true">🚨</span>
              <div className="flex-1 min-w-0">
                <p className="font-semibold text-sm" data-testid="pace-alert-toast-title">{paceAlertToast.title}</p>
                {paceAlertToast.body_parsed.subscriber_name && (
                  <p className="text-xs text-red-100 mt-0.5" data-testid="pace-alert-toast-subscriber">
                    Subscriber: {paceAlertToast.body_parsed.subscriber_name}
                  </p>
                )}
                {paceAlertToast.body_parsed.responder_agency && (
                  <p className="text-xs text-red-100 mt-0.5" data-testid="pace-alert-toast-agency">
                    Agency: {paceAlertToast.body_parsed.responder_agency}
                  </p>
                )}
                {paceAlertToast.body_parsed.location && (
                  <p className="text-xs text-red-100 mt-0.5" data-testid="pace-alert-toast-location">
                    Location: {paceAlertToast.body_parsed.location}
                  </p>
                )}
                {paceAlertToast.body_parsed.scan_timestamp && (
                  <p className="text-xs text-red-100 mt-0.5" data-testid="pace-alert-toast-time">
                    Time: {new Date(paceAlertToast.body_parsed.scan_timestamp).toLocaleString()}
                  </p>
                )}
                {paceAlertToast.body_parsed.trustee_role && (
                  <p className="text-xs text-red-100 mt-0.5" data-testid="pace-alert-toast-role">
                    Your role: {paceAlertToast.body_parsed.trustee_role}
                  </p>
                )}
              </div>
            </div>
            <button
              onClick={dismissPaceAlertToast}
              className="text-red-200 hover:text-white flex-shrink-0 text-lg leading-none font-bold"
              aria-label="Dismiss PACE alert"
              data-testid="dismiss-pace-alert-toast"
            >
              ×
            </button>
          </div>
        </div>
      )}

      {/* Main Content */}
      <main id="main-content" tabIndex={-1} className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mb-6">
          <h1 className="text-2xl font-bold text-gray-900">
            Welcome{user?.first_name ? `, ${user.first_name}` : ''}
          </h1>
          <p className="text-gray-600 mt-1">
            Your consent-bound health identity dashboard
          </p>
        </div>

        {/* Feature #398: Next Steps / Onboarding Progress Panel */}
        {acceptedCount < 4 && (
          <div className="mb-6 p-4 bg-amber-50 rounded-xl border border-amber-200" data-testid="onboarding-next-steps">
            <h2 className="text-sm font-semibold text-amber-800 mb-3">🚀 Getting Started — Complete Your Setup</h2>
            <ol className="space-y-2" aria-label="Onboarding next steps">
              {/* Step 1: Profile */}
              <li className="flex items-center gap-3">
                <span className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold flex-shrink-0 ${user?.first_name ? 'bg-emerald-500 text-white' : 'bg-amber-400 text-white'}`}>
                  {user?.first_name ? '✓' : '1'}
                </span>
                <span className="text-sm text-gray-700 flex-1">
                  {user?.first_name ? (
                    <span className="text-emerald-700 font-medium">Profile set up</span>
                  ) : (
                    <>
                      <Link to="/onboarding" className="text-sky-700 font-medium hover:underline" data-testid="next-step-profile-link">
                        Complete your profile
                      </Link>
                      <span className="text-gray-500"> — name, health info, emergency contacts</span>
                    </>
                  )}
                </span>
              </li>
              {/* Step 2: Nominate trustees */}
              <li className="flex items-center gap-3">
                <span className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold flex-shrink-0 ${filledCount >= 4 ? 'bg-emerald-500 text-white' : filledCount > 0 ? 'bg-amber-400 text-white' : 'bg-gray-300 text-gray-600'}`}>
                  {filledCount >= 4 ? '✓' : '2'}
                </span>
                <span className="text-sm text-gray-700 flex-1">
                  {filledCount >= 4 ? (
                    <span className="text-emerald-700 font-medium">4 PACE trustees nominated</span>
                  ) : (
                    <>
                      <Link to="/pace" className="text-sky-700 font-medium hover:underline" data-testid="next-step-pace-link">
                        Nominate 4 PACE trustees
                      </Link>
                      <span className="text-gray-500"> - {filledCount > 0 ? `${filledCount} of 4 nominated` : 'Primary, Alternate, Contingent, Emergency'}</span>
                    </>
                  )}
                </span>
              </li>
              {/* Step 3: Await acceptance */}
              <li className="flex items-center gap-3">
                <span className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold flex-shrink-0 ${acceptedCount >= 4 ? 'bg-emerald-500 text-white' : filledCount > 0 ? 'bg-amber-400 text-white' : 'bg-gray-300 text-gray-600'}`}>
                  {acceptedCount >= 4 ? '✓' : '3'}
                </span>
                <span className="text-sm text-gray-700 flex-1">
                  {acceptedCount >= 4 ? (
                    <span className="text-emerald-700 font-medium">All trustees accepted</span>
                  ) : filledCount > 0 ? (
                    <span data-testid="next-step-await-trustees">
                      <span className="font-medium text-amber-700">Awaiting trustee acceptance</span>
                      <span className="text-gray-500"> — {acceptedCount} of 4 accepted so far</span>
                    </span>
                  ) : (
                    <span className="text-gray-400">Await trustee acceptance (after step 2)</span>
                  )}
                </span>
              </li>
            </ol>
          </div>
        )}

        {/* Feature #305: Tab Navigation */}
        <div className="mb-6 border-b border-gray-200" data-testid="dashboard-tabs">
          <nav className="-mb-px flex gap-1" aria-label="Dashboard sections">
            {DASHBOARD_TABS.map(tab => (
              <button
                key={tab.key}
                onClick={() => setActiveTab(tab.key)}
                data-testid={`tab-${tab.key}`}
                aria-selected={activeTab === tab.key}
                className={`px-4 py-2 text-sm font-medium rounded-t-lg border-b-2 transition-colors focus:outline-none focus:ring-2 focus:ring-sky-500 focus:ring-offset-1 ${
                  activeTab === tab.key
                    ? 'border-sky-600 text-sky-700 bg-sky-50'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                {tab.label}
              </button>
            ))}
          </nav>
        </div>

        {/* DID Info — shown on all tabs */}
        {user?.did && (
          <div className="mb-6 p-4 bg-sky-50 rounded-lg border border-sky-200">
            <p className="text-sm text-sky-800">
              <span className="font-medium">Your DID:</span>{' '}
              <code className="text-xs bg-sky-100 px-2 py-0.5 rounded">{user.did}</code>
            </p>
          </div>
        )}

        {/* === OVERVIEW TAB === */}
        {activeTab === 'overview' && (
          <div data-testid="tab-content-overview">
        {/* Status Overview */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          {/* PACE Status */}
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-4" data-testid="pace-status">
            <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">PACE Trustees</p>
            <p className={`text-2xl font-bold ${acceptedCount === 4 ? 'text-emerald-600' : 'text-sky-700'}`}>{acceptedCount}<span className="text-base font-normal text-gray-400">/4</span></p>
            <p className={`text-xs mt-1 font-medium ${acceptedCount === 4 ? 'text-emerald-600' : 'text-gray-500'}`} data-testid="pace-completion-label">
              {acceptedCount === 4 ? '✓ Complete' : filledCount > 0 ? `${filledCount} nominated` : 'No trustees yet'}
            </p>
          </div>
          {/* Card Status */}
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-4" data-testid="card-status">
            <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Emergency Card</p>
            <div className="flex items-start justify-between gap-2">
              <div>
                <p className={`text-lg font-bold ${cardIsActive ? 'text-emerald-600' : cardIssued ? 'text-amber-600' : 'text-gray-400'}`} data-testid="card-status-label">
                  {cardIsActive ? 'Active' : cardIssued ? 'Pending' : 'Not Issued'}
                </p>
                {cardIsActive && cardStatus?.card?.issued_at && (
                  <p className="text-xs text-gray-500 mt-0.5" data-testid="card-issue-date">
                    Issued: {new Date(cardStatus.card.issued_at).toLocaleDateString()}
                  </p>
                )}
                <p className="text-xs text-gray-500 mt-1">{cardIsActive ? 'QR/NFC ready' : cardIssued ? 'Processing...' : 'Issue your card'}</p>
              </div>
              {cardIsActive && cardStatus?.card?.qr_data && (
                <img
                  src={cardStatus.card.qr_data}
                  alt="Emergency Card QR Code Preview - Click to view full card"
                  data-testid="card-qr-thumbnail"
                  className="w-12 h-12 object-contain rounded border border-gray-200 flex-shrink-0 cursor-pointer"
                  onClick={() => navigate('/card')}
                  title="Click to view your emergency card"
                />
              )}
            </div>
          </div>
          {/* Health Vault */}
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-4" data-testid="vault-completeness">
            <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Health Vault</p>
            <p className="text-2xl font-bold text-amber-600">
              {vaultCompleteness !== null ? `${vaultCompleteness}%` : '—'}
            </p>
            <p className="text-xs text-gray-500 mt-1">{recordCount !== null ? `${recordCount} record${recordCount !== 1 ? 's' : ''}` : 'Loading...'}</p>
          </div>
          {/* 0dentity Score */}
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-4 cursor-pointer hover:border-purple-200 transition" onClick={() => navigate('/odentity')} data-testid="odentity-score-widget">
            <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">0dentity Score</p>
            <p className="text-2xl font-bold text-purple-600">
              {overallScore !== null ? overallScore : '—'}
            </p>
            <p className="text-xs text-gray-500 mt-1">Identity completeness</p>
          </div>
        </div>

        {/* Quick Actions Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <div onClick={() => navigate('/pace')} className="cursor-pointer">
            <DashboardCard
              title="PACE Trustees"
              description="Appoint 4 PACE trustees to protect your identity"
              icon="shield"
              status={`${acceptedCount} of 4 accepted`}
              color="sky"
            />
          </div>
          <div onClick={() => navigate('/card')} className="cursor-pointer">
            <DashboardCard
              title="Emergency Card"
              description="Generate your QR/NFC emergency card"
              icon="card"
              status={cardIsActive ? 'Active' : cardIssued ? 'Pending' : 'Issue card'}
              color="emerald"
            />
          </div>
          <div onClick={() => navigate('/records')} className="cursor-pointer">
            <DashboardCard
              title="Health Vault"
              description="Manage your medical records and credentials"
              icon="health"
              status={recordCount !== null ? `${recordCount} record${recordCount !== 1 ? 's' : ''}` : 'Upload records'}
              color="amber"
            />
          </div>
          <div onClick={() => navigate('/credentials')} className="cursor-pointer">
            <DashboardCard
              title="Credential Vault"
              description="Upload and manage insurance cards and credentials"
              icon="credential"
              status="Manage credentials"
              color="indigo"
            />
          </div>
          <div onClick={() => navigate('/odentity')} className="cursor-pointer">
            <DashboardCard
              title="0dentity Score"
              description="View your multi-dimensional identity completeness"
              icon="score"
              status={overallScore !== null ? `Score: ${overallScore}` : 'View score'}
              color="purple"
            />
          </div>
          <div onClick={() => navigate('/notifications')} className="cursor-pointer">
            <DashboardCard
              title="Notifications"
              description="View your alerts, PACE notifications, and system messages"
              icon="bell"
              status="View notifications"
              color="rose"
            />
          </div>
          <div onClick={() => navigate('/research')} className="cursor-pointer">
            <DashboardCard
              title="Research Bridge"
              description="Opt into clinical trial matching via CyberMedica integration"
              icon="research"
              status="Manage opt-in"
              color="teal"
            />
          </div>
          <div onClick={() => navigate('/scan-history')} className="cursor-pointer" data-testid="scan-history-card">
            <DashboardCard
              title="Scan History"
              description="View all emergency card scans by first responders"
              icon="scan"
              status="View scan history"
              color="orange"
            />
          </div>
          <div onClick={() => navigate('/alert-history')} className="cursor-pointer" data-testid="alert-history-card">
            <DashboardCard
              title="Alert History"
              description="View PACE alerts and emergency notifications"
              icon="alert"
              status="View alert history"
              color="rose"
            />
          </div>
        </div>
          </div>
        )}

        {/* === PACE TAB === */}
        {activeTab === 'pace' && (
          <div data-testid="tab-content-pace">
        {/* PACE Trustees Section */}
        <div className="mt-2">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-lg font-semibold text-gray-900">PACE Trustees</h2>
              <p className="text-sm text-gray-500 mt-1">
                Appoint 4 P.A.C.E. contacts (Primary, Alternate, Contingent, Emergency) for emergency readiness.
                Email, SMS, and Copy link invitations are managed from the P.A.C.E. page.
              </p>
            </div>
          </div>

          {nominateSuccess && (
            <div className="mb-4 p-3 bg-emerald-50 border border-emerald-200 text-emerald-700 rounded-lg text-sm" data-testid="nominate-success">
              {nominateSuccess}
            </div>
          )}
          {nominateError && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" data-testid="nominate-error">
              {nominateError}
            </div>
          )}
          {replaceSuccess && (
            <div className="mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-700 rounded-lg text-sm" data-testid="replace-success">
              {replaceSuccess}
            </div>
          )}
          {replaceError && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" data-testid="replace-error">
              {replaceError}
            </div>
          )}

          {loadingTrustees ? (
            <p className="text-gray-500 text-sm">Loading trustees...</p>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4" data-testid="pace-slots">
              {PACE_ROLES.map((role) => {
                const trustee = getTrusteeForRole(role.key);
                const isNominating = nominateRole === role.key;
                const colorMap = {
                  sky: { bg: 'bg-sky-50', border: 'border-sky-200', text: 'text-sky-700', badge: 'bg-sky-100 text-sky-800', letterBg: 'bg-sky-600' },
                  emerald: { bg: 'bg-emerald-50', border: 'border-emerald-200', text: 'text-emerald-700', badge: 'bg-emerald-100 text-emerald-800', letterBg: 'bg-emerald-600' },
                  amber: { bg: 'bg-amber-50', border: 'border-amber-200', text: 'text-amber-700', badge: 'bg-amber-100 text-amber-800', letterBg: 'bg-amber-600' },
                  rose: { bg: 'bg-rose-50', border: 'border-rose-200', text: 'text-rose-700', badge: 'bg-rose-100 text-rose-800', letterBg: 'bg-rose-600' },
                };
                const colors = colorMap[role.color] || colorMap.sky;

                return (
                  <div
                    key={role.key}
                    className={`p-4 rounded-lg border-2 ${trustee ? colors.bg + ' ' + colors.border : 'bg-white border-gray-200 border-dashed'}`}
                    data-testid={`pace-slot-${role.key}`}
                  >
                    <div className="flex items-start gap-3">
                      <div className={`w-10 h-10 rounded-full flex items-center justify-center text-white font-bold text-lg ${trustee ? colors.letterBg : 'bg-gray-300'}`}>
                        {role.letter}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <h3 className="font-semibold text-gray-900">{role.name}</h3>
                          {trustee && (
                            <span className={`px-2 py-0.5 text-xs rounded-full ${trustee.status === 'accepted' ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`} data-testid={`trustee-status-${role.key}`}>
                              {trustee.status === 'accepted' ? 'Accepted' : 'Pending'}
                            </span>
                          )}
                        </div>
                        <p className="text-xs text-gray-500 mt-1">{role.description}</p>

                        {trustee ? (
                          <div className="mt-2">
                            <p className="text-sm font-medium text-gray-700" data-testid={`trustee-email-${role.key}`}>{trustee.email}</p>
                            {trustee.status === 'accepted' && trustee.shard_ref && (
                              <p className="text-xs text-gray-500 mt-1" data-testid={`trustee-shard-${role.key}`}>
                                Key shard: <code className="bg-gray-100 px-1 rounded">{trustee.shard_ref}</code>
                              </p>
                            )}
                            {trustee.accepted_at && (
                              <p className="text-xs text-gray-400 mt-0.5" data-testid={`trustee-accepted-at-${role.key}`}>
                                Accepted: {new Date(trustee.accepted_at).toLocaleDateString()}
                              </p>
                            )}
                            {trustee.status === 'accepted' && (
                              replaceMode === trustee.id ? (
                                <div className="mt-2">
                                  <input
                                    type="email"
                                    value={replaceEmail}
                                    onChange={e => setReplaceEmail(e.target.value)}
                                    className="w-full px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-orange-400"
                                    placeholder="new.trustee@example.com"
                                    data-testid={`replace-email-input-${role.key}`}
                                  />
                                  <div className="flex gap-2 mt-1">
                                    <button
                                      onClick={() => handleReplace(trustee.id)}
                                      disabled={replaceSubmitting || !replaceEmail}
                                      className="px-3 py-1 bg-orange-600 text-white text-xs rounded hover:bg-orange-700 disabled:opacity-50"
                                      data-testid={`confirm-replace-btn-${role.key}`}
                                    >
                                      {replaceSubmitting ? 'Initiating...' : 'Confirm Replace'}
                                    </button>
                                    <button
                                      onClick={() => { setReplaceMode(null); setReplaceEmail(''); setReplaceError(''); }}
                                      className="px-3 py-1 text-xs text-gray-500 hover:text-gray-700"
                                    >Cancel</button>
                                  </div>
                                </div>
                              ) : (
                                <button
                                  onClick={() => { setReplaceMode(trustee.id); setReplaceEmail(''); setReplaceError(''); setReplaceSuccess(''); }}
                                  className="mt-2 px-3 py-1 text-xs bg-orange-100 text-orange-700 rounded hover:bg-orange-200 transition"
                                  data-testid={`replace-btn-${role.key}`}
                                >
                                  Replace Trustee
                                </button>
                              )
                            )}
                          </div>
                        ) : isNominating ? (
                          <div className="mt-3">
                            <div className="flex gap-2">
                              <input
                                type="email"
                                value={trusteeEmails[role.key] || ''}
                                onChange={(e) => setTrusteeEmails(prev => ({ ...prev, [role.key]: e.target.value }))}
                                className="flex-1 px-3 py-2 text-sm border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                                placeholder="trustee@example.com"
                                data-testid={`trustee-email-input-${role.key}`}
                              />
                              <button
                                onClick={() => handleNominate(role.key)}
                                disabled={nominateSubmitting || !trusteeEmails[role.key]}
                                className="px-4 py-2 bg-sky-600 text-white text-sm rounded-lg hover:bg-sky-700 transition disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
                                data-testid={`send-invitation-btn-${role.key}`}
                              >
                                {nominateSubmitting ? 'Sending...' : 'Send Invitation'}
                              </button>
                            </div>
                            <button
                              onClick={() => setNominateRole(null)}
                              className="mt-2 text-xs text-gray-500 hover:text-gray-700"
                            >
                              Cancel
                            </button>
                          </div>
                        ) : (
                          <button
                            onClick={() => { setNominateRole(role.key); setNominateError(''); setNominateSuccess(''); }}
                            className="mt-3 px-4 py-2 bg-sky-600 text-white text-sm rounded-lg hover:bg-sky-700 transition"
                            data-testid={`nominate-btn-${role.key}`}
                          >
                            Nominate {role.name} Trustee
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {filledCount === 4 && (
            <div className="mt-4 p-4 bg-emerald-50 border border-emerald-200 rounded-lg text-center" data-testid="all-trustees-filled">
              <p className="text-emerald-700 font-medium">All 4 PACE trustees have been nominated!</p>
              <p className="text-emerald-600 text-sm mt-1">Your identity is protected by the PACE trust network.</p>
            </div>
          )}

          {/* VSS Ceremony Status */}
          {vssCeremony && (
            <div className="mt-4 p-4 bg-sky-50 border border-sky-200 rounded-lg" data-testid="vss-ceremony-status">
              <div className="flex items-center gap-2 mb-2">
                <div className="w-3 h-3 rounded-full bg-emerald-500"></div>
                <h3 className="font-semibold text-sky-900">VSS Key Shard Ceremony Complete</h3>
              </div>
              <div className="grid grid-cols-2 gap-2 text-sm">
                <div>
                  <span className="text-sky-700">Threshold:</span>{' '}
                  <span className="font-medium text-sky-800" data-testid="vss-threshold">{vssCeremony.threshold}-of-{vssCeremony.total_shares}</span>
                </div>
                <div>
                  <span className="text-sky-700">Ceremony Type:</span>{' '}
                  <span className="font-medium text-sky-800" data-testid="vss-ceremony-type">{vssCeremony.ceremony_type}</span>
                </div>
                <div className="col-span-2">
                  <span className="text-sky-700">Master Key Hash:</span>{' '}
                  <code className="text-xs bg-sky-100 px-1 rounded text-sky-800" data-testid="vss-master-key-hash">
                    {vssCeremony.master_key_hash ? vssCeremony.master_key_hash.substring(0, 16) + '...' : 'N/A'}
                  </code>
                </div>
                <div>
                  <span className="text-sky-700">Status:</span>{' '}
                  <span className="px-2 py-0.5 text-xs rounded-full bg-emerald-100 text-emerald-700" data-testid="vss-status">{vssCeremony.status}</span>
                </div>
                <div>
                  <span className="text-sky-700">Generated:</span>{' '}
                  <span className="text-sky-800" data-testid="vss-created-at">{new Date(vssCeremony.created_at).toLocaleString()}</span>
                </div>
              </div>
              <p className="text-xs text-sky-700 mt-2">
                Verifiable Secret Sharing ensures your identity key is split among {vssCeremony.total_shares} trustees.
                Any {vssCeremony.threshold} trustees can reconstruct the key for identity recovery.
              </p>
            </div>
          )}

          {/* Governance Workflows */}
          {governanceWorkflows.length > 0 && (
            <div className="mt-6" data-testid="governance-workflows">
              <h3 className="font-semibold text-gray-900 mb-3">Governance Workflows</h3>
              <div className="space-y-3">
                {governanceWorkflows.map(wf => (
                  <div
                    key={wf.id}
                    className={`p-4 rounded-lg border ${wf.status === 'approved' ? 'bg-emerald-50 border-emerald-200' : 'bg-orange-50 border-orange-200'}`}
                    data-testid={`workflow-${wf.id}`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <span className="font-medium text-sm text-gray-900">
                        {wf.workflow_type === 'trustee_replacement' ? '🔄 Trustee Replacement' :
                         wf.workflow_type === 'identity_recovery' ? '🔑 Identity Recovery' : wf.workflow_type}
                      </span>
                      <span className={`px-2 py-0.5 text-xs rounded-full ${wf.status === 'approved' ? 'bg-emerald-100 text-emerald-700' : 'bg-orange-100 text-orange-700'}`}
                        data-testid={`workflow-status-${wf.id}`}>
                        {wf.status}
                      </span>
                    </div>
                    <div className="text-xs text-gray-600 space-y-1">
                      <p>Signers: <span className="font-medium" data-testid={`workflow-signers-${wf.id}`}>{wf.current_signers}/{wf.required_signers}</span></p>
                      {wf.metadata?.old_trustee_email && (
                        <p>Replacing: <span className="font-medium">{wf.metadata.old_trustee_email}</span> ({wf.metadata.old_trustee_role})</p>
                      )}
                      {wf.metadata?.new_trustee_email && (
                        <p>New trustee: <span className="font-medium">{wf.metadata.new_trustee_email}</span></p>
                      )}
                      {wf.completed_at && (
                        <p className="text-gray-400">Completed: {new Date(wf.completed_at).toLocaleDateString()}</p>
                      )}
                      {wf.deadline_at && !wf.completed_at && (
                        <p className="text-orange-600">Deadline: {new Date(wf.deadline_at).toLocaleDateString()}</p>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
          </div>
        )}

        {/* === CARD TAB === */}
        {activeTab === 'card' && (
          <div data-testid="tab-content-card">
            <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-2">Emergency Card</h2>
              <p className="text-sm text-gray-500 mb-4">
                Your QR/NFC emergency card gives first responders instant, consent-gated access to your critical health information.
              </p>
              <div className="flex items-center gap-4 mb-4">
                <div className={`px-3 py-1.5 rounded-full text-sm font-medium ${
                  cardIsActive ? 'bg-emerald-100 text-emerald-700' :
                  cardIssued ? 'bg-amber-100 text-amber-700' :
                  'bg-gray-100 text-gray-600'
                }`} data-testid="card-tab-status">
                  {cardIsActive ? '✓ Active' : cardIssued ? '⏳ Pending' : 'Not Issued'}
                </div>
                {cardIsActive && cardStatus?.card?.issued_at && (
                  <span className="text-sm text-gray-500">
                    Issued: {new Date(cardStatus.card.issued_at).toLocaleDateString()}
                  </span>
                )}
              </div>
              {cardIsActive && cardStatus?.card?.qr_data && (
                <div className="mb-4">
                  <img
                    src={cardStatus.card.qr_data}
                    alt="Emergency Card QR Code"
                    className="w-32 h-32 object-contain rounded border border-gray-200"
                  />
                </div>
              )}
              <button
                onClick={() => navigate('/card')}
                className="px-5 py-2.5 bg-emerald-600 text-white text-sm rounded-lg hover:bg-emerald-700 transition font-medium"
                data-testid="go-to-card-btn"
              >
                {cardIsActive ? 'Manage Card' : 'Issue Emergency Card'}
              </button>
            </div>
          </div>
        )}

        {/* === HEALTH VAULT TAB === */}
        {activeTab === 'vault' && (
          <div data-testid="tab-content-vault">
            <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-2">Health Vault</h2>
              <p className="text-sm text-gray-500 mb-4">
                Your complete medical jacket in encrypted LiveSafe storage. EXOCHAIN anchoring remains pending adapter verification.
              </p>
              <div className="flex items-center gap-6 mb-4">
                <div>
                  <p className="text-3xl font-bold text-amber-600" data-testid="vault-tab-record-count">
                    {recordCount !== null ? recordCount : '—'}
                  </p>
                  <p className="text-sm text-gray-500">Records</p>
                </div>
                <div>
                  <p className="text-3xl font-bold text-amber-600" data-testid="vault-tab-completeness">
                    {vaultCompleteness !== null ? `${vaultCompleteness}%` : '—'}
                  </p>
                  <p className="text-sm text-gray-500">Completeness</p>
                </div>
              </div>
              {/* Feature #401: Section-by-section completeness breakdown */}
              {vaultSections && (
                <div className="mb-4" data-testid="vault-sections-breakdown">
                  <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">Vault Sections</p>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                    {[
                      { key: 'profile', label: 'Profile (blood type & DOB)', testId: 'vault-section-profile' },
                      { key: 'allergies', label: 'Allergies', testId: 'vault-section-allergies' },
                      { key: 'medications', label: 'Medications', testId: 'vault-section-medications' },
                      { key: 'conditions', label: 'Conditions', testId: 'vault-section-conditions' },
                      { key: 'insurance_card', label: 'Insurance Card', testId: 'vault-section-insurance' },
                    ].map(({ key, label, testId }) => (
                      <div key={key} className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm ${vaultSections[key] ? 'bg-emerald-50 text-emerald-700' : 'bg-gray-50 text-gray-500'}`} data-testid={testId} data-complete={vaultSections[key] ? 'true' : 'false'}>
                        <span className="text-base">{vaultSections[key] ? '✓' : '○'}</span>
                        <span>{label}</span>
                        <span className="ml-auto font-medium">{vaultSections[key] ? '+20%' : '0%'}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              <button
                onClick={() => navigate('/records')}
                className="px-5 py-2.5 bg-amber-600 text-white text-sm rounded-lg hover:bg-amber-700 transition font-medium"
                data-testid="go-to-vault-btn"
              >
                Open Health Vault
              </button>
            </div>
          </div>
        )}

      </main>
    </div>
  );
}

function DashboardCard({ title, description, status, color }) {
  const colorMap = {
    sky: 'bg-sky-50 border-sky-200 text-sky-700',
    emerald: 'bg-emerald-50 border-emerald-200 text-emerald-700',
    amber: 'bg-amber-50 border-amber-200 text-amber-700',
    purple: 'bg-purple-50 border-purple-200 text-purple-700',
    indigo: 'bg-indigo-50 border-indigo-200 text-indigo-700',
    rose: 'bg-rose-50 border-rose-200 text-rose-700',
    teal: 'bg-teal-50 border-teal-200 text-teal-700',
    orange: 'bg-orange-50 border-orange-200 text-orange-700',
  };

  return (
    <div className={`p-6 rounded-xl border-2 ${colorMap[color] || colorMap.sky}`}>
      <h2 className="text-lg font-semibold mb-2">{title}</h2>
      <p className="text-sm opacity-80 mb-4">{description}</p>
      <span className="text-xs font-medium px-2 py-1 bg-white bg-opacity-60 rounded-full">
        {status}
      </span>
    </div>
  );
}

export default Dashboard;
