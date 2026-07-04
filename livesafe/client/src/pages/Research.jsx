import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

export default function Research() {
  const { user } = useAuth();
  const navigate = useNavigate();

  const [optInStatus, setOptInStatus] = useState(null);
  const [auditTrail, setAuditTrail] = useState([]);
  const [trials, setTrials] = useState(null);
  const [trialConsents, setTrialConsents] = useState([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [trialsLoading, setTrialsLoading] = useState(false);
  const [trialActionLoading, setTrialActionLoading] = useState({});
  const [withdrawConfirm, setWithdrawConfirm] = useState(null);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [trialsError, setTrialsError] = useState('');

  const fetchOptInStatus = async () => {
    try {
      const res = await api.get('/research/opt-in');
      setOptInStatus(res.data);
    } catch (err) {
      console.error('Failed to fetch opt-in status:', err);
      setError('Failed to load research preferences');
    }
  };

  const fetchAuditTrail = async () => {
    try {
      const res = await api.get('/research/audit');
      setAuditTrail(res.data || []);
    } catch (err) {
      console.error('Failed to fetch research audit trail:', err);
    }
  };

  const fetchTrialConsents = async () => {
    try {
      const res = await api.get('/research/trial-consents');
      setTrialConsents(res.data || []);
    } catch (err) {
      console.error('Failed to fetch trial consents:', err);
    }
  };

  const handleEnrollInTrial = async (trialId, trialTitle) => {
    setTrialActionLoading(prev => ({ ...prev, [trialId]: 'enrolling' }));
    setMessage('');
    setError('');
    try {
      const res = await api.post('/research/trials/' + trialId + '/consent');
      setMessage('Successfully enrolled in: ' + trialTitle);
      await Promise.all([fetchTrialConsents(), fetchAuditTrail()]);
    } catch (err) {
      if (err.response?.status === 409) {
        setMessage('Already enrolled in: ' + trialTitle);
      } else {
        setError(err.response?.data?.error || 'Failed to enroll in trial');
      }
    } finally {
      setTrialActionLoading(prev => ({ ...prev, [trialId]: null }));
    }
  };

  const handleWithdrawFromTrial = async (trialId, trialTitle) => {
    setWithdrawConfirm(null);
    setTrialActionLoading(prev => ({ ...prev, [trialId]: 'withdrawing' }));
    setMessage('');
    setError('');
    try {
      const res = await api.delete('/research/trials/' + trialId + '/consent');
      setMessage('Successfully withdrawn from: ' + trialTitle);
      await Promise.all([fetchTrialConsents(), fetchAuditTrail()]);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to withdraw from trial');
    } finally {
      setTrialActionLoading(prev => ({ ...prev, [trialId]: null }));
    }
  };

  const fetchTrials = async () => {
    setTrialsLoading(true);
    setTrialsError('');
    try {
      const res = await api.get('/research/trials');
      setTrials(res.data);
      await Promise.all([fetchAuditTrail(), fetchTrialConsents()]);
    } catch (err) {
      if (err.response?.status === 403) {
        setTrialsError('Please opt in to clinical trial matching first.');
      } else {
        setTrialsError(err.response?.data?.error || 'Failed to check trial eligibility');
      }
    } finally {
      setTrialsLoading(false);
    }
  };

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      await Promise.all([fetchOptInStatus(), fetchAuditTrail(), fetchTrialConsents()]);
      setLoading(false);
    };
    load();
  }, []);

  const handleOptIn = async () => {
    setActionLoading(true);
    setMessage('');
    setError('');
    try {
      const res = await api.post('/research/opt-in');
      setMessage(res.data.message || 'Successfully opted into clinical trial matching');
      await Promise.all([fetchOptInStatus(), fetchAuditTrail()]);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to opt into clinical trial matching');
    } finally {
      setActionLoading(false);
    }
  };

  const handleOptOut = async () => {
    setActionLoading(true);
    setMessage('');
    setError('');
    try {
      const res = await api.delete('/research/opt-in');
      setMessage(res.data.message || 'Successfully opted out of clinical trial matching');
      setTrials(null);
      await Promise.all([fetchOptInStatus(), fetchAuditTrail()]);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to opt out of clinical trial matching');
    } finally {
      setActionLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading research preferences...</p>
        </div>
      </div>
    );
  }

  const isOptedIn = optInStatus?.opted_in === true;

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main className="max-w-4xl mx-auto px-4 py-8 space-y-6">

        {/* Info Banner */}
        <div className="bg-sky-50 border border-sky-200 rounded-xl p-5">
          <div className="flex items-start gap-3">
            <div className="w-10 h-10 rounded-full bg-sky-100 flex items-center justify-center flex-shrink-0">
              <svg className="w-5 h-5 text-sky-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.347.347a3.75 3.75 0 01-5.304-5.304l.347-.347z" />
              </svg>
            </div>
            <div>
              <h2 className="text-base font-semibold text-sky-900 mb-1">Clinical Trial Matching</h2>
              <p className="text-sm text-sky-800">
                Opt into the CyberMedica research bridge to be matched with relevant clinical trials.
                Your identity is protected — only de-identified health data is used for matching via
                zero-knowledge proofs. Your PHI is never exposed to research platforms.
              </p>
              <ul className="mt-2 text-xs text-sky-700 space-y-1">
                <li>• Matching uses ZK proofs (groth16) — your PHI stays private</li>
                <li>• Research platform receives only: eligible (true/false) + ZK proof reference</li>
                <li>• Consent stays in a local audit trail while EXOCHAIN adapter proof remains inactive</li>
                <li>• You can withdraw at any time</li>
              </ul>
            </div>
          </div>
        </div>

        {/* Current Status Card */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Your Opt-In Status</h2>

          <div className={`flex items-center gap-3 p-4 rounded-lg ${isOptedIn ? 'bg-emerald-50 border border-emerald-200' : 'bg-gray-50 border border-gray-200'}`}>
            <div className={`w-3 h-3 rounded-full ${isOptedIn ? 'bg-emerald-500' : 'bg-gray-400'}`}></div>
            <div className="flex-1">
              <p className={`font-medium ${isOptedIn ? 'text-emerald-800' : 'text-gray-600'}`}>
                {isOptedIn ? 'Opted In — Active' : 'Not Opted In'}
              </p>
              {isOptedIn && optInStatus?.opt_in_at && (
                <p className="text-xs text-emerald-600 mt-0.5">
                  Opted in on {new Date(optInStatus.opt_in_at).toLocaleDateString('en-US', {
                    year: 'numeric', month: 'long', day: 'numeric'
                  })}
                </p>
              )}
              {isOptedIn && optInStatus?.cybermedica_consent_ref && (
                <p className="text-xs text-emerald-600 mt-0.5">
                  CyberMedica Consent Ref: <span className="font-mono">{optInStatus.cybermedica_consent_ref}</span>
                </p>
              )}
              {!isOptedIn && optInStatus?.opt_out_at && (
                <p className="text-xs text-gray-500 mt-0.5">
                  Opted out on {new Date(optInStatus.opt_out_at).toLocaleDateString()}
                </p>
              )}
            </div>
            <div className={`px-3 py-1 rounded-full text-xs font-medium ${isOptedIn ? 'bg-emerald-100 text-emerald-700' : 'bg-gray-100 text-gray-600'}`}>
              {isOptedIn ? 'Active' : 'Inactive'}
            </div>
          </div>

          {/* Success/Error messages */}
          {message && (
            <div className="mt-4 p-3 bg-emerald-50 border border-emerald-200 rounded-lg text-sm text-emerald-800">
              {message}
            </div>
          )}
          {error && (
            <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-700">
              {error}
            </div>
          )}

          {/* Action buttons */}
          <div className="mt-5 space-y-3">
            {!isOptedIn ? (
              <button
                onClick={handleOptIn}
                disabled={actionLoading}
                className="w-full py-3 px-4 bg-sky-600 hover:bg-sky-700 disabled:bg-sky-300 text-white font-semibold rounded-lg transition-colors"
              >
                {actionLoading ? (
                  <span className="flex items-center justify-center gap-2">
                    <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                    Processing...
                  </span>
                ) : (
                  'Opt In to Clinical Trial Matching'
                )}
              </button>
            ) : (
              <>
                <button
                  onClick={fetchTrials}
                  disabled={trialsLoading}
                  className="w-full py-3 px-4 bg-sky-600 hover:bg-sky-700 disabled:bg-sky-300 text-white font-semibold rounded-lg transition-colors"
                >
                  {trialsLoading ? (
                    <span className="flex items-center justify-center gap-2">
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                      Checking Eligibility via ZK Proof...
                    </span>
                  ) : (
                    'Check Matched Clinical Trials'
                  )}
                </button>
                <button
                  onClick={handleOptOut}
                  disabled={actionLoading}
                  className="w-full py-3 px-4 bg-gray-100 hover:bg-gray-200 disabled:bg-gray-50 text-gray-700 font-semibold rounded-lg transition-colors border border-gray-200"
                >
                  {actionLoading ? (
                    <span className="flex items-center justify-center gap-2">
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-500"></div>
                      Processing...
                    </span>
                  ) : (
                    'Withdraw from Clinical Trial Matching'
                  )}
                </button>
              </>
            )}
          </div>
        </div>

        {/* Trial Matching Results */}
        {(trials || trialsError) && (
          <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">Matched Clinical Trials</h2>
              {trials && (
                <div className="flex items-center gap-2">
                  <span className="text-xs px-2 py-1 bg-emerald-100 text-emerald-700 rounded-full font-medium">
                    ZK-Protected
                  </span>
                  <span className="text-xs text-gray-500">
                    {trials.total_eligible} of {trials.total_checked} eligible
                  </span>
                </div>
              )}
            </div>

            {trialsError && (
              <p className="text-sm text-red-600">{trialsError}</p>
            )}

            {trials && (
              <>
                <div className="mb-4 p-3 bg-sky-50 border border-sky-200 rounded-lg text-xs text-sky-800 flex items-start gap-2">
                  <svg className="w-4 h-4 text-sky-600 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                  </svg>
                  <span>
                    <strong>PHI Protected:</strong> Eligibility was computed server-side via{' '}
                    <span className="font-mono">groth16-simulated</span> ZK proofs.
                    CyberMedica receives only the eligibility result + ZK proof reference —
                    zero personal health information is transmitted.
                  </span>
                </div>

                <div className="space-y-3">
                  {trials.trials.map((trial) => {
                    const consent = trialConsents.find(c => c.trial_id === trial.trial_id && c.status === 'active');
                    const withdrawnConsent = trialConsents.find(c => c.trial_id === trial.trial_id && c.status === 'withdrawn');
                    const isEnrolled = !!consent;
                    const isWithdrawn = !isEnrolled && !!withdrawnConsent;
                    const isActing = trialActionLoading[trial.trial_id];
                    return (
                      <div
                        key={trial.trial_id}
                        data-testid={`trial-card-${trial.trial_id}`}
                        className={`p-4 rounded-lg border ${isEnrolled ? 'bg-sky-50 border-sky-300' : isWithdrawn ? 'bg-orange-50 border-orange-200' : trial.eligible ? 'bg-emerald-50 border-emerald-200' : 'bg-gray-50 border-gray-200'}`}
                      >
                        <div className="flex items-start justify-between gap-3">
                          <div className="flex-1">
                            <div className="flex items-center gap-2 mb-1 flex-wrap">
                              <h3 className={`text-sm font-semibold ${isEnrolled ? 'text-sky-900' : isWithdrawn ? 'text-orange-800' : trial.eligible ? 'text-emerald-900' : 'text-gray-600'}`}>
                                {trial.title}
                              </h3>
                              <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${trial.eligible ? 'bg-emerald-100 text-emerald-700' : 'bg-gray-100 text-gray-500'}`}>
                                {trial.eligible ? 'Eligible' : 'Not Eligible'}
                              </span>
                              {isEnrolled && (
                                <span className="text-xs px-2 py-0.5 rounded-full font-medium bg-sky-100 text-sky-700">
                                  ✓ Enrolled
                                </span>
                              )}
                              {isWithdrawn && (
                                <span data-testid={`trial-withdrawn-${trial.trial_id}`} className="text-xs px-2 py-0.5 rounded-full font-medium bg-orange-100 text-orange-700">
                                  ⊘ Consent Revoked
                                </span>
                              )}
                            </div>
                            <p className="text-xs text-gray-500 mb-2">{trial.description}</p>
                            <div className="flex flex-wrap gap-3 text-xs text-gray-500">
                              <span>Phase: <span className="font-medium">{trial.phase}</span></span>
                              <span>Sponsor: <span className="font-medium">{trial.sponsor}</span></span>
                            </div>
                          </div>
                        </div>

                        {/* Enroll / Withdraw buttons */}
                        {trial.eligible && (
                          <div className="mt-3 flex items-center gap-2">
                            {!isEnrolled ? (
                              withdrawConfirm === trial.trial_id ? null : (
                                <button
                                  onClick={() => handleEnrollInTrial(trial.trial_id, trial.title)}
                                  disabled={!!isActing}
                                  className="text-xs px-3 py-1.5 bg-emerald-600 hover:bg-emerald-700 disabled:bg-emerald-300 text-white rounded-lg font-medium transition-colors"
                                >
                                  {isActing === 'enrolling' ? 'Enrolling...' : '+ Enroll in Trial'}
                                </button>
                              )
                            ) : (
                              withdrawConfirm === trial.trial_id ? (
                                <div className="flex items-center gap-2">
                                  <span className="text-xs text-red-700 font-medium">Confirm withdrawal?</span>
                                  <button
                                    onClick={() => handleWithdrawFromTrial(trial.trial_id, trial.title)}
                                    disabled={!!isActing}
                                    className="text-xs px-3 py-1 bg-red-600 hover:bg-red-700 text-white rounded font-medium"
                                  >
                                    {isActing === 'withdrawing' ? 'Withdrawing...' : 'Confirm Withdraw'}
                                  </button>
                                  <button
                                    onClick={() => setWithdrawConfirm(null)}
                                    className="text-xs px-3 py-1 bg-gray-200 hover:bg-gray-300 text-gray-700 rounded font-medium"
                                  >
                                    Cancel
                                  </button>
                                </div>
                              ) : (
                                <button
                                  onClick={() => setWithdrawConfirm(trial.trial_id)}
                                  disabled={!!isActing}
                                  className="text-xs px-3 py-1.5 bg-gray-100 hover:bg-red-50 hover:border-red-200 border border-gray-200 text-gray-600 hover:text-red-600 rounded-lg font-medium transition-colors"
                                >
                                  {isActing === 'withdrawing' ? 'Withdrawing...' : 'Withdraw from Trial'}
                                </button>
                              )
                            )}
                            {isEnrolled && consent?.consent_ref && (
                              <span className="text-xs text-gray-400">
                                Ref: <span className="font-mono">{consent.consent_ref}</span>
                              </span>
                            )}
                          </div>
                        )}

                        <div className="mt-3 pt-3 border-t border-current border-opacity-10">
                          <div className="flex items-center gap-2 text-xs">
                            <span className="text-gray-400">ZK Proof:</span>
                            <span className="font-mono text-gray-500 truncate">{trial.zk_proof_ref}</span>
                            <span className="bg-sky-100 text-sky-700 px-1.5 py-0.5 rounded text-xs">groth16</span>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </>
            )}
          </div>
        )}

        {/* Consent Scope */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-3">Consent Scope</h2>
          <div className="flex items-center gap-3 p-3 bg-amber-50 border border-amber-200 rounded-lg">
            <svg className="w-5 h-5 text-amber-600 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
            <div>
              <p className="text-sm font-medium text-amber-900">De-identified Trial Matching</p>
              <p className="text-xs text-amber-700 mt-0.5">
                Scope: <span className="font-mono">de_identified_trial_matching</span> — Only
                anonymized eligibility criteria are shared. No PHI is transmitted.
              </p>
            </div>
          </div>
          <p className="mt-3 text-xs text-gray-500">
            In this current LiveSafe surface, research consent uses consent type{' '}
            <span className="font-mono bg-gray-100 px-1 rounded">research_access</span> with purpose{' '}
            <span className="font-mono bg-gray-100 px-1 rounded">clinical_trial_participation</span>.
            Consent expires at trial end or upon withdrawal, and the audit trail remains local until a verified adapter path is invoked.
          </p>
        </div>

        {/* Audit Trail */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Research Consent Audit Trail</h2>
          {auditTrail.length === 0 ? (
            <p className="text-sm text-gray-500 text-center py-4">
              No research consent events yet.
            </p>
          ) : (
            <div className="space-y-3">
              {auditTrail.map((entry) => (
                <div key={entry.id} className="flex items-start gap-3 p-3 bg-gray-50 rounded-lg">
                  <div className={`w-2 h-2 mt-1.5 rounded-full flex-shrink-0 ${
                    entry.event_type === 'research_opt_in' ? 'bg-emerald-500' :
                    entry.event_type === 'trial_consent_granted' ? 'bg-sky-500' :
                    entry.event_type === 'trial_consent_withdrawn' ? 'bg-red-400' :
                    entry.event_type === 'trial_eligibility_check' ? 'bg-blue-400' :
                    'bg-gray-400'
                  }`}></div>
                  <div className="flex-1">
                    <div className="flex items-center justify-between">
                      <span className={`text-sm font-medium ${
                        entry.event_type === 'research_opt_in' ? 'text-emerald-700' :
                        entry.event_type === 'trial_consent_granted' ? 'text-sky-700' :
                        entry.event_type === 'trial_consent_withdrawn' ? 'text-red-600' :
                        entry.event_type === 'trial_eligibility_check' ? 'text-blue-700' :
                        'text-gray-600'
                      }`}>
                        {entry.event_type === 'research_opt_in' ? 'Opted In to Trial Matching' :
                         entry.event_type === 'research_opt_out' ? 'Opted Out of Trial Matching' :
                         entry.event_type === 'trial_eligibility_check' ? 'Eligibility Check (ZK)' :
                         entry.event_type === 'trial_consent_granted' ? 'Trial Consent Granted' :
                         entry.event_type === 'trial_consent_withdrawn' ? 'Trial Consent Withdrawn' :
                         entry.event_type}
                      </span>
                      <span className="text-xs text-gray-400">
                        {new Date(entry.created_at).toLocaleString()}
                      </span>
                    </div>
                    <p className="text-xs text-gray-500 mt-0.5">
                      Scope: {entry.scope}
                    </p>
                    {entry.details?.cybermedica_consent_ref && (
                      <p className="text-xs text-gray-500">
                        Ref: <span className="font-mono">{entry.details.cybermedica_consent_ref}</span>
                      </p>
                    )}
                    {entry.event_type === 'trial_eligibility_check' && entry.details && (
                      <p className="text-xs text-blue-600">
                        Checked {entry.details.trials_checked} trials — PHI exposed: {String(entry.details.phi_exposed)}
                      </p>
                    )}
                    {(entry.event_type === 'trial_consent_granted' || entry.event_type === 'trial_consent_withdrawn') && entry.details && (
                      <div className="mt-1">
                        <p className="text-xs text-gray-600 font-medium">{entry.details.trial_title}</p>
                        {entry.details.consent_ref && (
                          <p className="text-xs text-gray-400">Ref: <span className="font-mono">{entry.details.consent_ref}</span></p>
                        )}
                        {entry.details.subscriber_did && (
                          <p className="text-xs text-gray-400">Subscriber DID: <span className="font-mono text-xs">{entry.details.subscriber_did.substring(0, 40)}...</span></p>
                        )}
                      </div>
                    )}
                  </div>
                  <div className="text-xs text-gray-400 bg-gray-200 px-2 py-0.5 rounded">
                    Local Audit
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* EXOCHAIN notice */}
        <div className="text-center text-xs text-gray-400 pb-4">
          Research consent events remain in a local audit trail while EXOCHAIN anchoring stays inactive until a verified adapter path is invoked.
          This surface does not claim immutable EXOCHAIN enforcement before adapter proof passes.
        </div>

      </main>
    </div>
  );
}
