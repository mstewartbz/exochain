import React, { useState, useEffect } from 'react';
import { useNavigate, useParams, Link } from 'react-router-dom';
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

function TrusteeSubscriberDetail() {
  const navigate = useNavigate();
  const { subscriberDid } = useParams();
  const [detail, setDetail] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [overrideLoading, setOverrideLoading] = useState(false);
  const [overrideResult, setOverrideResult] = useState(null);
  const [overrideError, setOverrideError] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('livesafe_trustee_token');
    if (!token) {
      navigate('/trustee/login');
      return;
    }

    api.get(`/pace/subscriber/${encodeURIComponent(subscriberDid)}/details`, {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then((res) => {
        setDetail(res.data);
        setLoading(false);
      })
      .catch((err) => {
        if (err.response?.status === 401 || err.response?.status === 403) {
          localStorage.removeItem('livesafe_trustee_token');
          localStorage.removeItem('livesafe_trustee_user');
          navigate('/trustee/login');
        } else {
          setError(err.response?.data?.error || 'Failed to load subscriber details');
          setLoading(false);
        }
      });
  }, [navigate, subscriberDid]);

  const handleSignOut = () => {
    localStorage.removeItem('livesafe_trustee_token');
    localStorage.removeItem('livesafe_trustee_user');
    navigate('/trustee/login');
  };

  const handleInitiateEmergencyOverride = async () => {
    const token = localStorage.getItem('livesafe_trustee_token');
    const userStr = localStorage.getItem('livesafe_trustee_user');
    if (!token || !userStr) { navigate('/trustee/login'); return; }
    const trusteeUser = JSON.parse(userStr);

    setOverrideLoading(true);
    setOverrideError('');
    setOverrideResult(null);

    try {
      const res = await api.post('/pace/governance/emergency-override', {
        trustee_did: trusteeUser.did || undefined,
        trustee_email: trusteeUser.email,
        subscriber_did: subscriberDid,
        reason: 'Trustee-initiated emergency access override',
      }, { headers: { Authorization: `Bearer ${token}` } });
      setOverrideResult(res.data);
    } catch (err) {
      setOverrideError(err.response?.data?.error || 'Failed to initiate emergency override');
    } finally {
      setOverrideLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading subscriber details...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <p className="text-red-600 mb-4">{error}</p>
          <Link to="/trustee/dashboard" className="text-sky-700 hover:underline">← Back to Dashboard</Link>
        </div>
      </div>
    );
  }

  const paceComplete = detail?.pace_trustees?.length === 4 && detail.pace_trustees.every(t => t.status === 'accepted');
  const paceAccepted = detail?.pace_trustees?.filter(t => t.status === 'accepted').length || 0;

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Navigation */}
      <nav className="bg-white border-b border-gray-200 shadow-sm">
        <div className="max-w-4xl mx-auto px-4 py-3 flex items-center justify-between">
          <h1 className="text-xl font-bold text-sky-700">
            LiveSafe<span className="text-emerald-600">.ai</span>
          </h1>
          <div className="flex items-center gap-4">
            <Link
              to="/trustee/dashboard"
              className="text-sm text-sky-700 hover:text-sky-800"
              data-testid="back-to-dashboard-link"
            >
              ← Dashboard
            </Link>
            <button
              onClick={handleSignOut}
              className="text-sm text-gray-500 hover:text-gray-700 border border-gray-300 px-3 py-1 rounded"
            >
              Sign Out
            </button>
          </div>
        </div>
      </nav>

      <main className="max-w-4xl mx-auto px-4 py-8">
        <div className="mb-6">
          <h2 className="text-2xl font-bold text-gray-900" data-testid="subscriber-detail-heading">
            Subscriber Details
          </h2>
          <p className="text-gray-500 text-sm mt-1">Protected party information for your P.A.C.E. role</p>
        </div>

        {/* Subscriber Overview */}
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-6 mb-6" data-testid="subscriber-overview">
          <div className="flex items-start justify-between">
            <div>
              <h3 className="text-xl font-semibold text-gray-900" data-testid="subscriber-detail-name">
                {detail?.subscriber_name}
              </h3>
              <p className="text-sm text-gray-500 mt-1">
                <code className="bg-gray-100 px-2 py-0.5 rounded text-xs" data-testid="subscriber-detail-did">{detail?.subscriber_did}</code>
              </p>
            </div>
            <div>
              <span
                data-testid="subscriber-detail-status"
                className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm font-medium border
                  ${detail?.subscriber_status === 'protected'
                    ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                    : detail?.subscriber_status === 'active'
                    ? 'bg-sky-50 text-sky-700 border-sky-200'
                    : 'bg-yellow-50 text-yellow-700 border-yellow-200'
                  }`}
              >
                <span className={`inline-block w-2 h-2 rounded-full
                  ${detail?.subscriber_status === 'protected' ? 'bg-emerald-500'
                    : detail?.subscriber_status === 'active' ? 'bg-sky-500'
                    : 'bg-yellow-500'}`}
                />
                {detail?.subscriber_status === 'protected' ? 'Fully Protected'
                  : detail?.subscriber_status === 'active' ? 'Active'
                  : 'Unverified'}
              </span>
            </div>
          </div>

          <div className="mt-4 grid grid-cols-2 gap-4">
            <div className="p-3 bg-gray-50 rounded-lg">
              <p className="text-xs text-gray-500 font-medium uppercase tracking-wide">Email Verified</p>
              <p className="text-sm font-semibold mt-1 text-gray-800" data-testid="subscriber-email-verified">
                {detail?.email_verified ? '✅ Verified' : '⚠️ Not Verified'}
              </p>
            </div>
            <div className="p-3 bg-gray-50 rounded-lg">
              <p className="text-xs text-gray-500 font-medium uppercase tracking-wide">PACE Trust Level</p>
              <p className="text-sm font-semibold mt-1 text-gray-800" data-testid="subscriber-pace-level">
                {paceAccepted}/4 contacts active
                {paceComplete && <span className="ml-2 text-emerald-600">✅ Complete</span>}
              </p>
            </div>
          </div>
        </div>

        {/* P.A.C.E. Contacts */}
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-6 mb-6" data-testid="subscriber-pace-trustees">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">P.A.C.E. Safety Circle</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {detail?.pace_trustees?.map((trustee) => {
              const role = normalizeRole(trustee.role);
              return (
                <div
                  key={role}
                  className="p-4 rounded-lg border border-gray-100 bg-gray-50"
                  data-testid={`pace-trustee-${role}`}
                >
                <div className="flex items-center gap-3">
                  <div className={`w-10 h-10 rounded-full ${PACE_ROLE_BG[role] || 'bg-gray-600'} flex items-center justify-center text-white font-bold`}>
                    {PACE_ROLE_LETTERS[role] || '?'}
                  </div>
                  <div>
                    <p className="font-medium text-gray-900 capitalize text-sm">{role} Contact</p>
                    <span
                      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border ${
                        trustee.status === 'accepted'
                          ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                          : trustee.status === 'pending'
                          ? 'bg-yellow-50 text-yellow-700 border-yellow-200'
                          : 'bg-red-50 text-red-700 border-red-200'
                      }`}
                      data-testid={`pace-trustee-${role}-status`}
                    >
                      <span className={`inline-block w-1.5 h-1.5 rounded-full ${
                        trustee.status === 'accepted' ? 'bg-emerald-500'
                        : trustee.status === 'pending' ? 'bg-yellow-500'
                        : 'bg-red-500'}`}
                      />
                      {trustee.status === 'accepted' ? 'Active' : trustee.status === 'pending' ? 'Pending' : trustee.status}
                    </span>
                  </div>
                </div>
                {trustee.is_me && (
                  <p className="text-xs text-sky-700 font-medium mt-2" data-testid={`pace-trustee-${role}-isyou`}>
                    This is your role
                  </p>
                )}
                {trustee.accepted_at && (
                  <p className="text-xs text-gray-400 mt-1">
                    Accepted: {new Date(trustee.accepted_at).toLocaleDateString()}
                  </p>
                )}
                </div>
              );
            })}
          </div>
        </div>

        {/* My Role Details */}
        {detail?.my_trusteeship && (
          <div className="bg-sky-50 rounded-xl border border-sky-200 p-6" data-testid="my-trusteeship-details">
            <h3 className="text-lg font-semibold text-sky-900 mb-3">Your P.A.C.E. Role Details</h3>
            <div className="space-y-2 text-sm text-sky-800">
              <p>
                <span className="font-medium">Role:</span>{' '}
                <span className="capitalize" data-testid="my-trusteeship-role">{normalizeRole(detail.my_trusteeship.role)}</span>
              </p>
              <p>
                <span className="font-medium">VSS Shard:</span>{' '}
                <span
                  className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium border ${
                    detail.my_trusteeship.has_vss_shard
                      ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                      : 'bg-red-50 text-red-700 border-red-200'
                  }`}
                  data-testid="my-trusteeship-shard"
                >
                  <span
                    className={`inline-block w-1.5 h-1.5 rounded-full ${
                      detail.my_trusteeship.has_vss_shard ? 'bg-emerald-500' : 'bg-red-500'
                    }`}
                  />
                  {detail.my_trusteeship.shard_status === 'present' ? 'Present' : 'Missing'}
                </span>
              </p>
              <p>
                <span className="font-medium">Accepted:</span>{' '}
                <span data-testid="my-trusteeship-date">
                  {detail.my_trusteeship.accepted_at ? new Date(detail.my_trusteeship.accepted_at).toLocaleDateString() : 'N/A'}
                </span>
              </p>
            </div>
          </div>
        )}

        {/* Emergency Access Override Section */}
        <div className="mt-6 bg-rose-50 rounded-xl border border-rose-200 p-6" data-testid="emergency-override-section">
          <h3 className="text-lg font-semibold text-rose-900 mb-2">🚨 Emergency Access Override</h3>
          <p className="text-sm text-rose-700 mb-4">
            If emergency medical personnel need access to this subscriber's complete medical records, you can initiate an emergency access override.
            This requires approval from 2 of the 4 P.A.C.E. contacts and expires after 1 hour.
          </p>

          {overrideError && (
            <div className="mb-3 p-3 bg-red-50 border border-red-200 text-red-700 rounded text-sm" data-testid="override-error">
              {overrideError}
            </div>
          )}

          {overrideResult ? (
            <div className="p-4 bg-white rounded-lg border border-rose-200" data-testid="override-result">
              <p className="text-sm font-semibold text-emerald-700 mb-2">✅ Emergency override workflow initiated</p>
              <div className="text-xs text-gray-600 space-y-1">
                <p><span className="font-medium">Workflow ID:</span> <code data-testid="override-workflow-id">{overrideResult.workflow_id}</code></p>
                <p><span className="font-medium">Status:</span> <span data-testid="override-status">{overrideResult.status}</span></p>
                <p><span className="font-medium">Approvals:</span> <span data-testid="override-signers">{overrideResult.current_signers}/{overrideResult.required_signers}</span></p>
                <p><span className="font-medium">Deadline:</span> <span data-testid="override-deadline">{overrideResult.deadline_at ? new Date(overrideResult.deadline_at).toLocaleTimeString() : 'N/A'}</span></p>
                <p><span className="font-medium">Trustees notified:</span> <span data-testid="override-trustees-notified">{overrideResult.trustees_notified}</span></p>
              </div>
              <p className="mt-2 text-xs text-gray-500">{overrideResult.message}</p>
            </div>
          ) : (
            <button
              onClick={handleInitiateEmergencyOverride}
              disabled={overrideLoading}
              className="px-4 py-2 bg-rose-600 text-white text-sm rounded-lg hover:bg-rose-700 disabled:opacity-50 transition font-medium"
              data-testid="initiate-emergency-override-btn"
            >
              {overrideLoading ? 'Initiating...' : '🚨 Initiate Emergency Access Override'}
            </button>
          )}
        </div>
      </main>
    </div>
  );
}

export default TrusteeSubscriberDetail;
