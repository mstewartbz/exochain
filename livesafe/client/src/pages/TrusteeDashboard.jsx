import React, { useState, useEffect } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import api from '../services/api';

const PACE_ROLE_COLORS = {
  primary: 'bg-sky-100 text-sky-800 border-sky-200',
  alternate: 'bg-emerald-100 text-emerald-800 border-emerald-200',
  contingent: 'bg-amber-100 text-amber-800 border-amber-200',
  emergency: 'bg-rose-100 text-rose-800 border-rose-200',
};

const PACE_ROLE_LETTERS = {
  primary: 'P',
  alternate: 'A',
  contingent: 'C',
  emergency: 'E',
};

const PACE_ROLE_BG = {
  primary: 'bg-sky-600',
  alternate: 'bg-emerald-600',
  contingent: 'bg-amber-600',
  emergency: 'bg-rose-600',
};

function normalizeRole(role) {
  return role === 'custodial' ? 'contingent' : role;
}

function TrusteeDashboard() {
  const navigate = useNavigate();
  const [profile, setProfile] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [pendingWorkflows, setPendingWorkflows] = useState([]);
  const [signingWorkflow, setSigningWorkflow] = useState(null);
  const [signError, setSignError] = useState('');
  const [signSuccess, setSignSuccess] = useState('');
  const [alertNotifications, setAlertNotifications] = useState([]);
  const [alertsLoading, setAlertsLoading] = useState(false);
  const [respondingAlert, setRespondingAlert] = useState(null);
  const [respondSuccess, setRespondSuccess] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('livesafe_trustee_token');
    if (!token) {
      navigate('/trustee/login');
      return;
    }

    api.get('/auth/trustee/me', {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then((res) => {
        setProfile(res.data);
        setLoading(false);
        // Fetch pending governance workflows and alert history
        if (res.data?.did) {
          fetchPendingWorkflows(res.data.did);
          fetchAlertHistory(res.data.did);
        }
      })
      .catch((err) => {
        if (err.response?.status === 401 || err.response?.status === 403) {
          localStorage.removeItem('livesafe_trustee_token');
          localStorage.removeItem('livesafe_trustee_user');
          navigate('/trustee/login');
        } else {
          setError(err.response?.data?.error || 'Failed to load profile');
          setLoading(false);
        }
      });
  }, [navigate]);

  const fetchPendingWorkflows = async (trusteeDid) => {
    try {
      const res = await api.get(`/pace/governance/trustee/${trusteeDid}`);
      setPendingWorkflows(res.data || []);
    } catch (err) {
      console.error('Failed to fetch governance workflows:', err);
    }
  };

  const fetchAlertHistory = async (trusteeDid) => {
    try {
      setAlertsLoading(true);
      const res = await api.get(`/alerts/trustee-notifications/${encodeURIComponent(trusteeDid)}`);
      setAlertNotifications(res.data.notifications || []);
    } catch (err) {
      console.error('Failed to fetch alert history:', err);
    } finally {
      setAlertsLoading(false);
    }
  };

  const handleRespondAlert = async (notificationId, responseStatus) => {
    if (!profile?.did) return;
    setRespondingAlert(notificationId);
    setRespondSuccess('');
    try {
      await api.post(`/alerts/respond/${notificationId}`, {
        trustee_did: profile.did,
        response_status: responseStatus,
        response_message: responseStatus === 'available' ? "I'm available and on my way" : 'Currently unavailable',
      });
      setRespondSuccess(`Response recorded: ${responseStatus}`);
      // Refresh alerts to show updated response
      if (profile?.did) fetchAlertHistory(profile.did);
    } catch (err) {
      console.error('Failed to respond to alert:', err);
    } finally {
      setRespondingAlert(null);
    }
  };

  const handleSignOut = () => {
    localStorage.removeItem('livesafe_trustee_token');
    localStorage.removeItem('livesafe_trustee_user');
    navigate('/trustee/login');
  };

  const handleSignWorkflow = async (workflowId, workflowType) => {
    if (!profile?.did) return;
    setSignError('');
    setSignSuccess('');
    setSigningWorkflow(workflowId);

    try {
      let res;
      if (workflowType === 'identity_recovery') {
        res = await api.post(`/pace/recovery/${workflowId}/sign`, { trustee_did: profile.did });
      } else {
        res = await api.post(`/pace/governance/${workflowId}/sign`, { trustee_did: profile.did });
      }

      const msg = res.data.quorum_met
        ? `✅ Quorum met! ${res.data.message}`
        : `Signed (${res.data.current_signers}/${res.data.required_signers}). ${res.data.message}`;
      setSignSuccess(msg);

      // Refresh pending workflows
      fetchPendingWorkflows(profile.did);
    } catch (err) {
      setSignError(err.response?.data?.error || 'Failed to sign workflow');
    } finally {
      setSigningWorkflow(null);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading trustee dashboard...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center text-red-600">{error}</div>
      </div>
    );
  }

  const trusteeships = profile?.trusteeships || [];

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Navigation */}
      <nav className="bg-white border-b border-gray-200 shadow-sm">
        <div className="max-w-5xl mx-auto px-4 py-3 flex items-center justify-between">
          <h1 className="text-xl font-bold text-sky-700">
            LiveSafe<span className="text-emerald-600">.ai</span>
          </h1>
          <div className="flex items-center gap-4">
            <span className="text-sm text-gray-600" data-testid="trustee-email">{profile?.email}</span>
            <button
              onClick={handleSignOut}
              className="text-sm text-gray-500 hover:text-gray-700 border border-gray-300 px-3 py-1 rounded"
              data-testid="trustee-signout-btn"
            >
              Sign Out
            </button>
          </div>
        </div>
      </nav>

      {/* Main content */}
      <main className="max-w-5xl mx-auto px-4 py-8">
        <div className="mb-6">
          <h2 className="text-2xl font-bold text-gray-900" data-testid="trustee-welcome">
            Welcome{profile?.first_name ? `, ${profile.first_name}` : ''}
          </h2>
          <p className="text-gray-600">Your P.A.C.E. Contact Dashboard</p>
        </div>

        <p className="text-sm text-gray-500 mb-6">
          <span className="font-medium">Your DID:</span>{' '}
          <code className="bg-gray-100 px-2 py-0.5 rounded text-xs" data-testid="trustee-did">{profile?.did}</code>
        </p>

        {/* Trusteeships summary */}
        <div className="mb-8 p-4 bg-white rounded-xl border border-gray-200 shadow-sm">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">P.A.C.E. roles</h3>
          <p className="text-sm text-gray-600 mb-4">
            You are serving as a P.A.C.E. contact for{' '}
            <span className="font-bold text-sky-700" data-testid="trusteeship-count">{trusteeships.length}</span>{' '}
            subscriber{trusteeships.length !== 1 ? 's' : ''}.
          </p>
        </div>

        {/* Pending Governance Actions */}
        {(pendingWorkflows.length > 0 || signSuccess || signError) && (
          <div className="mb-8 p-4 bg-orange-50 border border-orange-200 rounded-xl shadow-sm" data-testid="pending-governance">
            <h3 className="text-lg font-semibold text-orange-900 mb-3">⚠️ Pending Governance Actions</h3>
            {signSuccess && (
              <div className="mb-3 p-3 bg-emerald-50 border border-emerald-200 text-emerald-700 rounded text-sm" data-testid="sign-success">
                {signSuccess}
              </div>
            )}
            {signError && (
              <div className="mb-3 p-3 bg-red-50 border border-red-200 text-red-700 rounded text-sm" data-testid="sign-error">
                {signError}
              </div>
            )}
            {pendingWorkflows.length === 0 && <p className="text-sm text-gray-500">No pending workflows requiring your signature.</p>}
            <div className="space-y-3">
              {pendingWorkflows.map(wf => (
                <div key={wf.id} className="p-4 bg-white rounded-lg border border-orange-100" data-testid={`pending-workflow-${wf.id}`}>
                  <div className="flex items-center justify-between mb-2">
                    <div>
                      <span className="font-medium text-sm text-gray-900">
                        {wf.workflow_type === 'trustee_replacement' ? '🔄 Trustee Replacement Request' :
                         wf.workflow_type === 'identity_recovery' ? '🔑 Identity Recovery Request' :
                         wf.workflow_type === 'emergency_access_override' ? '🚨 Emergency Expanded Access Request' :
                         wf.workflow_type}
                      </span>
                      <p className="text-xs text-gray-500 mt-0.5">
                        Signers: {wf.current_signers}/{wf.required_signers} •{' '}
                        Deadline: {new Date(wf.deadline_at).toLocaleDateString()}
                      </p>
                      {wf.metadata?.old_trustee_email && (
                        <p className="text-xs text-gray-600 mt-1">
                          Replacing: {wf.metadata.old_trustee_email} → {wf.metadata.new_trustee_email}
                        </p>
                      )}
                      {wf.workflow_type === 'emergency_access_override' && (
                        <p className="text-xs text-orange-700 mt-1 font-medium">
                          ⚠️ A first responder is requesting expanded access to a subscriber's full medical records. 2 trustee approvals required.
                        </p>
                      )}
                    </div>
                    <button
                      onClick={() => handleSignWorkflow(wf.id, wf.workflow_type)}
                      disabled={signingWorkflow === wf.id}
                      className="px-4 py-2 bg-orange-600 text-white text-sm rounded-lg hover:bg-orange-700 disabled:opacity-50 transition"
                      data-testid={`sign-workflow-btn-${wf.id}`}
                    >
                      {signingWorkflow === wf.id ? 'Signing...' : 'Approve'}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Trusteeships list */}
        <div className="space-y-4" data-testid="trusteeships-list">
              {trusteeships.map((t, idx) => {
                const role = normalizeRole(t.role);
                return (
            <div
              key={t.id}
              className="p-5 bg-white rounded-xl border border-gray-200 shadow-sm"
              data-testid={`trusteeship-${idx}`}
            >
              <div className="flex items-start gap-4">
                <div className={`w-12 h-12 rounded-full ${PACE_ROLE_BG[role] || 'bg-gray-600'} flex items-center justify-center text-white font-bold text-xl`}>
                  {PACE_ROLE_LETTERS[role] || '?'}
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <h4 className="font-semibold text-gray-900 capitalize" data-testid={`trusteeship-${idx}-role`}>
                      {role} Contact
                    </h4>
                    <span className={`px-2 py-0.5 text-xs font-medium rounded-full border ${PACE_ROLE_COLORS[role] || 'bg-gray-100 text-gray-800'}`}>
                      {PACE_ROLE_LETTERS[role]}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 mb-2">
                    <p className="text-sm text-gray-700">
                      <span className="font-medium">Subscriber:</span>{' '}
                      <span data-testid={`trusteeship-${idx}-subscriber`}>{t.subscriber_name}</span>
                    </p>
                    {/* Subscriber status badge */}
                    <span
                      data-testid={`trusteeship-${idx}-subscriber-status`}
                      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border
                        ${t.subscriber_status === 'protected'
                          ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                          : t.subscriber_status === 'active'
                          ? 'bg-sky-50 text-sky-700 border-sky-200'
                          : 'bg-yellow-50 text-yellow-700 border-yellow-200'
                        }`}
                    >
                      <span className={`inline-block w-1.5 h-1.5 rounded-full
                        ${t.subscriber_status === 'protected' ? 'bg-emerald-500' : t.subscriber_status === 'active' ? 'bg-sky-500' : 'bg-yellow-500'}`}
                      />
                      {t.subscriber_status === 'protected' ? 'Fully Protected' : t.subscriber_status === 'active' ? 'Active' : 'Unverified'}
                    </span>
                  </div>
                  <div className="text-xs text-gray-500 space-y-1">
                    <p>
                      <span className="font-medium">Subscriber DID:</span>{' '}
                      <code className="bg-gray-50 px-1 rounded">{t.subscriber_did}</code>
                    </p>
                    <div className="flex items-center gap-2">
                      <span className="font-medium">VSS Shard:</span>{' '}
                      <span
                        data-testid={`trusteeship-${idx}-shard-health`}
                        className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs font-medium
                          ${t.has_vss_shard
                            ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                            : 'bg-red-50 text-red-700 border border-red-200'
                          }`}
                      >
                        <span className={`inline-block w-1.5 h-1.5 rounded-full ${t.has_vss_shard ? 'bg-emerald-500' : 'bg-red-500'}`} />
                        {t.shard_status === 'present' ? 'Present' : 'Missing'}
                      </span>
                    </div>
                    <p>
                      <span className="font-medium">PACE Trust:</span>{' '}
                      <span data-testid={`trusteeship-${idx}-pace-count`}>{t.subscriber_pace_count}/4 contacts active</span>
                    </p>
                    <p>
                      <span className="font-medium">Accepted:</span>{' '}
                      <span data-testid={`trusteeship-${idx}-date`}>{new Date(t.accepted_at).toLocaleDateString()}</span>
                    </p>
                  </div>
                  <div className="mt-3">
                    <Link
                      to={`/trustee/subscriber/${encodeURIComponent(t.subscriber_did)}`}
                      className="inline-flex items-center gap-1 px-3 py-1.5 bg-sky-600 text-white text-sm rounded-lg hover:bg-sky-700 transition"
                      data-testid={`trusteeship-${idx}-view-details-btn`}
                    >
                      View Subscriber Details →
                    </Link>
                  </div>
                </div>
              </div>
            </div>
                );
              })}
        </div>

        {trusteeships.length === 0 && (
          <div className="text-center py-12 text-gray-500">
            <p>No active trusteeships found.</p>
          </div>
        )}

        {/* Alert History Section */}
        <div className="mt-8" data-testid="trustee-alert-history">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Alert History</h3>
            {alertNotifications.length > 0 && (
              <span className="text-sm text-gray-500">
                {alertNotifications.length} alert{alertNotifications.length !== 1 ? 's' : ''}
              </span>
            )}
          </div>
          {respondSuccess && (
            <div className="mb-3 p-3 bg-emerald-50 border border-emerald-200 text-emerald-700 rounded text-sm" data-testid="respond-success">
              ✅ {respondSuccess}
            </div>
          )}

          {alertsLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500"></div>
            </div>
          ) : alertNotifications.length === 0 ? (
            <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-500">
              <div className="text-3xl mb-2">🔔</div>
              <p>No alerts received yet.</p>
              <p className="text-sm mt-1">You'll receive PACE alerts when a subscriber's card is scanned.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {alertNotifications.map((notif) => (
                <div
                  key={notif.id}
                  className={`bg-white rounded-xl border p-4 ${
                    notif.alert_type === 'pace_alert'
                      ? 'border-red-200'
                      : notif.alert_type === 'governance_approval'
                      ? 'border-orange-200'
                      : 'border-gray-200'
                  }`}
                  data-testid={`trustee-alert-${notif.id}`}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="text-lg">
                          {notif.alert_type === 'pace_alert' ? '🚨' :
                           notif.alert_type === 'governance_approval' ? '⚠️' : '📋'}
                        </span>
                        <h4 className="font-semibold text-sm text-gray-900"
                          data-testid={`trustee-alert-${notif.id}-title`}>
                          {notif.title}
                        </h4>
                        {/* Alert type */}
                        <span className="px-2 py-0.5 text-xs rounded-full bg-gray-100 text-gray-600"
                          data-testid={`trustee-alert-${notif.id}-type`}>
                          {notif.type_label}
                        </span>
                        {/* Response status */}
                        <span
                          className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                            notif.response_status === 'acknowledged'
                              ? 'bg-emerald-100 text-emerald-700'
                              : 'bg-sky-100 text-sky-700'
                          }`}
                          data-testid={`trustee-alert-${notif.id}-status`}
                        >
                          {notif.response_status === 'acknowledged' ? 'Acknowledged' : 'Sent'}
                        </span>
                      </div>

                      {/* Time */}
                      <p className="text-sm text-gray-600 mt-1">
                        <span className="font-medium">Time:</span>{' '}
                        <span data-testid={`trustee-alert-${notif.id}-time`}>
                          {new Date(notif.time).toLocaleString()}
                        </span>
                      </p>

                      {/* Channel */}
                      {notif.channel && (
                        <p className="text-xs text-gray-500 mt-0.5">
                          <span className="font-medium">Channel:</span>{' '}
                          <span className="capitalize">{notif.channel}</span>
                        </p>
                      )}

                      {/* Alert details */}
                      {notif.details && typeof notif.details === 'object' && (
                        <div className="mt-1 text-xs text-gray-500">
                          {notif.details.subscriber_name && (
                            <p><span className="font-medium">Subscriber:</span> {notif.details.subscriber_name}</p>
                          )}
                          {notif.details.location && (
                            <p><span className="font-medium">Location:</span> {notif.details.location}</p>
                          )}
                          {notif.details.responder_agency && (
                            <p><span className="font-medium">Agency:</span> {notif.details.responder_agency}</p>
                          )}
                        </div>
                      )}

                      {/* Trustee response for PACE alerts */}
                      {notif.alert_type === 'pace_alert' && (
                        <div className="mt-3">
                          {notif.response ? (
                            <div className="flex items-center gap-2">
                              <span className={`px-2 py-1 text-xs rounded-full font-medium ${
                                notif.response === 'available'
                                  ? 'bg-emerald-100 text-emerald-700'
                                  : 'bg-gray-100 text-gray-600'
                              }`}
                              data-testid={`alert-response-${notif.id}`}>
                                {notif.response === 'available' ? "✅ You responded: I'm available" : `❌ You responded: ${notif.response}`}
                              </span>
                              {notif.responded_at && (
                                <span className="text-xs text-gray-400">
                                  at {new Date(notif.responded_at).toLocaleTimeString()}
                                </span>
                              )}
                            </div>
                          ) : (
                            <div className="flex items-center gap-2">
                              <button
                                onClick={() => handleRespondAlert(notif.id, 'available')}
                                disabled={respondingAlert === notif.id}
                                className="px-3 py-1.5 bg-emerald-600 text-white text-xs rounded-lg hover:bg-emerald-700 disabled:opacity-50 transition font-medium"
                                data-testid={`respond-available-${notif.id}`}
                              >
                                {respondingAlert === notif.id ? 'Sending...' : "✅ I'm available"}
                              </button>
                              <button
                                onClick={() => handleRespondAlert(notif.id, 'unavailable')}
                                disabled={respondingAlert === notif.id}
                                className="px-3 py-1.5 bg-gray-200 text-gray-700 text-xs rounded-lg hover:bg-gray-300 disabled:opacity-50 transition"
                                data-testid={`respond-unavailable-${notif.id}`}
                              >
                                Unavailable
                              </button>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </main>
    </div>
  );
}

export default TrusteeDashboard;
