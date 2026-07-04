import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
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

function ScopeBadge({ scope }) {
  const colors = {
    full_medical_record: 'bg-purple-100 text-purple-700',
    emergency_info: 'bg-red-100 text-red-700',
    allergies_medications: 'bg-orange-100 text-orange-700',
    lab_results: 'bg-blue-100 text-blue-700',
    imaging: 'bg-indigo-100 text-indigo-700',
    conditions: 'bg-yellow-100 text-yellow-700',
    prescriptions: 'bg-teal-100 text-teal-700',
  };
  const label = SCOPE_OPTIONS.find(o => o.value === scope)?.label || scope;
  return (
    <span className={`px-2 py-0.5 text-xs font-medium rounded-full ${colors[scope] || 'bg-gray-100 text-gray-700'}`}>
      {label}
    </span>
  );
}

function PatientDataViewer({ consent, providerToken, onClose }) {
  const [patientData, setPatientData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    const fetchData = async () => {
      try {
        const res = await api.get(`/consent/patient/${consent.subscriber_id}/data`, {
          headers: { Authorization: `Bearer ${providerToken}` }
        });
        setPatientData(res.data);
      } catch (err) {
        if (err.response?.status === 403) {
          setError('Access denied: ' + (err.response.data?.error || 'No active consent for this patient.'));
        } else {
          setError(err.response?.data?.error || 'Failed to load patient data');
        }
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, [consent.subscriber_id, providerToken]);

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4" data-testid="patient-detail-view">
      <div className="bg-white rounded-xl shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div className="p-6 border-b border-gray-200 flex items-center justify-between">
          <div>
            <h3 className="text-lg font-bold text-gray-900" data-testid="patient-detail-heading">Patient Data</h3>
            <p className="text-sm text-gray-500 mt-0.5">
              Access scope: <ScopeBadge scope={consent.scope} />
            </p>
          </div>
          <button
            onClick={onClose}
            className="flex items-center gap-1.5 px-3 py-2 text-sm font-medium text-gray-600 border border-gray-300 hover:bg-gray-100 rounded-lg transition"
            data-testid="back-to-patient-list"
            aria-label="Back to patient list"
          >
            ← Back to Patient List
          </button>
        </div>

        <div className="p-6">
          {loading && (
            <div className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500 mx-auto mb-3"></div>
              <p className="text-gray-500 text-sm">Loading consent-scoped patient data...</p>
            </div>
          )}

          {error && (
            <div className="p-4 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" data-testid="patient-data-error">
              🔒 {error}
            </div>
          )}

          {patientData && !loading && (
            <div className="space-y-5" data-testid="patient-data-container">
              {/* Consent info banner */}
              <div className="p-3 bg-sky-50 border border-sky-200 rounded-lg text-sm text-sky-700">
                <strong>Consent-scoped access</strong> — Only data within your granted scope is shown.
                {patientData.non_consented_fields && (
                  <p className="mt-1 text-xs text-sky-700">
                    Fields outside your scope (hidden): {patientData.non_consented_fields.join(', ')}
                  </p>
                )}
              </div>

              {/* Subscriber Info */}
              {patientData.subscriber && (
                <div data-testid="subscriber-info">
                  <h4 className="text-sm font-semibold text-gray-700 uppercase tracking-wide mb-2">Patient Information</h4>
                  <div className="bg-gray-50 rounded-lg p-4 grid grid-cols-2 gap-3 text-sm">
                    {patientData.subscriber.first_name && (
                      <div>
                        <p className="text-xs text-gray-500">Name</p>
                        <p className="font-medium">{patientData.subscriber.first_name} {patientData.subscriber.last_name}</p>
                      </div>
                    )}
                    {patientData.subscriber.date_of_birth && (
                      <div>
                        <p className="text-xs text-gray-500">Date of Birth</p>
                        <p className="font-medium">{new Date(patientData.subscriber.date_of_birth).toLocaleDateString()}</p>
                      </div>
                    )}
                    {patientData.subscriber.blood_type && (
                      <div>
                        <p className="text-xs text-gray-500">Blood Type</p>
                        <p className="font-medium">{patientData.subscriber.blood_type}</p>
                      </div>
                    )}
                    {patientData.subscriber.dnr_status && (
                      <div>
                        <p className="text-xs text-gray-500">DNR Status</p>
                        <p className="font-medium">{patientData.subscriber.dnr_status}</p>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* Allergies */}
              {patientData.allergies !== undefined && (
                <div data-testid="allergies-section">
                  <h4 className="text-sm font-semibold text-gray-700 uppercase tracking-wide mb-2">
                    Allergies ({patientData.allergies.length})
                  </h4>
                  {patientData.allergies.length === 0 ? (
                    <p className="text-sm text-gray-400 italic">No allergies recorded</p>
                  ) : (
                    <div className="space-y-2">
                      {patientData.allergies.map(a => (
                        <div key={a.id} className="p-3 bg-orange-50 border border-orange-100 rounded-lg text-sm">
                          <p className="font-medium text-orange-800">{a.allergy || a.allergen}</p>
                          {a.reaction && <p className="text-orange-600 text-xs mt-0.5">Reaction: {a.reaction}</p>}
                          {a.severity && <p className="text-orange-600 text-xs">Severity: {a.severity}</p>}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Medications */}
              {patientData.medications !== undefined && (
                <div data-testid="medications-section">
                  <h4 className="text-sm font-semibold text-gray-700 uppercase tracking-wide mb-2">
                    Medications ({patientData.medications.length})
                  </h4>
                  {patientData.medications.length === 0 ? (
                    <p className="text-sm text-gray-400 italic">No medications recorded</p>
                  ) : (
                    <div className="space-y-2">
                      {patientData.medications.map(m => (
                        <div key={m.id} className="p-3 bg-blue-50 border border-blue-100 rounded-lg text-sm">
                          <p className="font-medium text-blue-800">{m.medication || m.medication_name}</p>
                          {m.dosage && <p className="text-blue-600 text-xs mt-0.5">Dosage: {m.dosage}</p>}
                          {m.frequency && <p className="text-blue-600 text-xs">Frequency: {m.frequency}</p>}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Conditions */}
              {patientData.conditions !== undefined && (
                <div data-testid="conditions-section">
                  <h4 className="text-sm font-semibold text-gray-700 uppercase tracking-wide mb-2">
                    Medical Conditions ({patientData.conditions.length})
                  </h4>
                  {patientData.conditions.length === 0 ? (
                    <p className="text-sm text-gray-400 italic">No conditions recorded</p>
                  ) : (
                    <div className="space-y-2">
                      {patientData.conditions.map(c => (
                        <div key={c.id} className="p-3 bg-yellow-50 border border-yellow-100 rounded-lg text-sm">
                          <p className="font-medium text-yellow-800">{c.condition_name}</p>
                          {c.notes && <p className="text-yellow-600 text-xs mt-0.5">{c.notes}</p>}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Credentials (insurance) - only for full access */}
              {patientData.credentials !== undefined && (
                <div data-testid="credentials-section">
                  <h4 className="text-sm font-semibold text-gray-700 uppercase tracking-wide mb-2">
                    Insurance Credentials ({patientData.credentials.length})
                  </h4>
                  {patientData.credentials.length === 0 ? (
                    <p className="text-sm text-gray-400 italic">No credentials on file</p>
                  ) : (
                    <div className="space-y-2">
                      {patientData.credentials.map(c => (
                        <div key={c.id} className="p-3 bg-green-50 border border-green-100 rounded-lg text-sm">
                          <p className="font-medium text-green-800">{c.title || c.credential_type}</p>
                          {c.carrier && <p className="text-green-600 text-xs mt-0.5">Carrier: {c.carrier}</p>}
                          {c.member_id && <p className="text-green-600 text-xs">Member ID: {c.member_id}</p>}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Medical Records */}
              {patientData.medical_records !== undefined && (
                <div data-testid="medical-records-section">
                  <h4 className="text-sm font-semibold text-gray-700 uppercase tracking-wide mb-2">
                    Medical Records ({patientData.medical_records.length})
                  </h4>
                  {patientData.medical_records.length === 0 ? (
                    <p className="text-sm text-gray-400 italic">No medical records on file</p>
                  ) : (
                    <div className="space-y-2">
                      {patientData.medical_records.map(r => (
                        <div key={r.id} className="p-3 bg-gray-50 border border-gray-200 rounded-lg text-sm">
                          <p className="font-medium text-gray-800">{r.title}</p>
                          {r.record_type && <p className="text-gray-500 text-xs mt-0.5">Type: {r.record_type}</p>}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Scope note */}
              <div className="p-3 bg-gray-50 border border-gray-200 rounded-lg text-xs text-gray-500">
                <strong>Access scope:</strong> {consent.scope} | Consent granted: {new Date(consent.granted_at || consent.created_at).toLocaleDateString()}
                {consent.expires_at && ` | Expires: ${new Date(consent.expires_at).toLocaleDateString()}`}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ProviderDashboard() {
  const navigate = useNavigate();
  const [provider, setProvider] = useState(null);
  const [consents, setConsents] = useState([]);
  const [accessRequests, setAccessRequests] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // Patient data viewer state (Feature #102)
  const [viewingConsent, setViewingConsent] = useState(null);

  // Request additional access state (Feature #103)
  const [showRequestForm, setShowRequestForm] = useState(false);
  const [requestConsentId, setRequestConsentId] = useState('');
  const [requestScope, setRequestScope] = useState('');
  const [requestPurpose, setRequestPurpose] = useState('');
  const [requestMessage, setRequestMessage] = useState('');
  const [requesting, setRequesting] = useState(false);
  const [requestMsg, setRequestMsg] = useState('');
  const [requestError, setRequestError] = useState('');

  // Clinical notes state (Feature #107)
  const [clinicalNotes, setClinicalNotes] = useState([]);
  const [showNoteForm, setShowNoteForm] = useState(false);
  const [noteConsentId, setNoteConsentId] = useState('');
  const [noteText, setNoteText] = useState('');
  const [noteType, setNoteType] = useState('clinical_note');
  const [submittingNote, setSubmittingNote] = useState(false);
  const [noteMsg, setNoteMsg] = useState('');
  const [noteError, setNoteError] = useState('');

  // Feature #317: Refresh state
  const [refreshing, setRefreshing] = useState(false);

  // Feature #317: Extract fetchData as a reusable callback for manual refresh
  const fetchData = useCallback(async (token, isRefresh = false) => {
    if (isRefresh) setRefreshing(true);
    try {
      const res = await api.get('/auth/provider/me', {
        headers: { Authorization: `Bearer ${token}` }
      });
      setProvider(res.data);
      setConsents(res.data.consents || []);

      // Load existing access requests
      try {
        const reqRes = await api.get('/consent/access-requests/provider', {
          headers: { Authorization: `Bearer ${token}` }
        });
        setAccessRequests(reqRes.data || []);
      } catch (_) { /* non-fatal */ }

      // Load clinical notes (Feature #107)
      try {
        const notesRes = await api.get('/records/clinical-notes/provider', {
          headers: { Authorization: `Bearer ${token}` }
        });
        setClinicalNotes(notesRes.data?.notes || []);
      } catch (_) { /* non-fatal */ }
    } catch (err) {
      if (err.response?.status === 401) {
        localStorage.removeItem('livesafe_provider_token');
        localStorage.removeItem('livesafe_provider_user');
        navigate('/provider/login');
      } else {
        setError('Failed to load provider data.');
      }
    } finally {
      setLoading(false);
      if (isRefresh) setRefreshing(false);
    }
  }, [navigate]);

  useEffect(() => {
    const token = localStorage.getItem('livesafe_provider_token');
    if (!token) {
      navigate('/provider/login');
      return;
    }

    const storedUser = localStorage.getItem('livesafe_provider_user');
    if (storedUser) {
      try {
        setProvider(JSON.parse(storedUser));
      } catch (e) { /* ignore */ }
    }

    fetchData(token, false);
  }, [navigate, fetchData]);

  // Feature #317: Manual refresh handler
  const handleRefreshConsents = () => {
    const token = localStorage.getItem('livesafe_provider_token');
    if (token) fetchData(token, true);
  };

  const handleLogout = () => {
    localStorage.removeItem('livesafe_provider_token');
    localStorage.removeItem('livesafe_provider_user');
    navigate('/provider/login');
  };

  const handleRequestAdditionalAccess = async (e) => {
    e.preventDefault();
    if (!requestConsentId || !requestScope) {
      setRequestError('Please select a patient and specify the desired access scope');
      return;
    }

    // Find the consent to get subscriber_id
    const consent = activeConsents.find(c => String(c.id) === String(requestConsentId));
    if (!consent) {
      setRequestError('Patient consent not found');
      return;
    }

    const token = localStorage.getItem('livesafe_provider_token');
    setRequesting(true);
    setRequestMsg('');
    setRequestError('');

    try {
      await api.post('/consent/request-access', {
        subscriber_id: consent.subscriber_id,
        requested_scope: requestScope,
        purpose: requestPurpose || 'additional_care',
        message: requestMessage || null,
      }, {
        headers: { Authorization: `Bearer ${token}` }
      });

      setRequestMsg(`✅ Access request sent to ${consent.subscriber_name} for approval`);
      setShowRequestForm(false);
      setRequestConsentId('');
      setRequestScope('');
      setRequestPurpose('');
      setRequestMessage('');

      // Refresh access requests
      const reqRes = await api.get('/consent/access-requests/provider', {
        headers: { Authorization: `Bearer ${token}` }
      });
      setAccessRequests(reqRes.data || []);
      setTimeout(() => setRequestMsg(''), 6000);
    } catch (err) {
      setRequestError(err.response?.data?.error || 'Failed to send access request');
    } finally {
      setRequesting(false);
    }
  };

  const handleAddClinicalNote = async (e) => {
    e.preventDefault();
    if (!noteConsentId || !noteText.trim()) {
      setNoteError('Please select a patient and enter the clinical note text');
      return;
    }

    const consent = activeConsents.find(c => String(c.id) === String(noteConsentId));
    if (!consent) {
      setNoteError('Patient consent not found');
      return;
    }

    const token = localStorage.getItem('livesafe_provider_token');
    setSubmittingNote(true);
    setNoteMsg('');
    setNoteError('');

    try {
      await api.post('/records/clinical-notes', {
        subscriber_id: consent.subscriber_id,
        note_text: noteText.trim(),
        note_type: noteType,
      }, {
        headers: { Authorization: `Bearer ${token}` }
      });

      setNoteMsg(`✅ Clinical note submitted to ${consent.subscriber_name || 'patient'} for approval`);
      setShowNoteForm(false);
      setNoteText('');
      setNoteConsentId('');
      setNoteType('clinical_note');

      // Refresh clinical notes
      const notesRes = await api.get('/records/clinical-notes/provider', {
        headers: { Authorization: `Bearer ${token}` }
      });
      setClinicalNotes(notesRes.data?.notes || []);
      setTimeout(() => setNoteMsg(''), 8000);
    } catch (err) {
      setNoteError(err.response?.data?.error || 'Failed to submit clinical note');
    } finally {
      setSubmittingNote(false);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading provider portal...</p>
        </div>
      </div>
    );
  }

  if (!provider) {
    navigate('/provider/login');
    return null;
  }

  const activeConsents = consents.filter(c => {
    if (c.revoked_at) return false;
    if (c.expires_at && new Date(c.expires_at) < new Date()) return false;
    return true;
  });

  const providerToken = localStorage.getItem('livesafe_provider_token');

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Patient Data Modal */}
      {viewingConsent && (
        <PatientDataViewer
          consent={viewingConsent}
          providerToken={providerToken}
          onClose={() => setViewingConsent(null)}
        />
      )}

      {/* Navigation */}
      <nav className="bg-white shadow-sm border-b">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center gap-3">
              <h1 className="text-xl font-bold text-sky-700">
                LiveSafe<span className="text-emerald-600">.ai</span>
              </h1>
              <span className="text-sm text-gray-400">›</span>
              <span className="text-sm font-medium text-gray-700">Provider Portal</span>
            </div>
            <div className="flex items-center gap-4">
              <span className="text-sm text-gray-600">{provider.email}</span>
              <button
                onClick={handleLogout}
                className="px-4 py-2 text-sm font-medium text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 transition"
                data-testid="provider-logout-btn"
              >
                Sign Out
              </button>
            </div>
          </div>
        </div>
      </nav>

      <main className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {error && (
          <div className="mb-6 p-4 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm">
            {error}
          </div>
        )}

        {/* Provider Profile Card */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-2xl font-bold text-gray-900" data-testid="provider-name">
                {provider.provider_name || provider.email}
              </h2>
              <p className="text-gray-600 mt-1" data-testid="provider-facility">{provider.facility}</p>
              {provider.specialty && (
                <p className="text-sm text-gray-500 mt-1">{provider.specialty}</p>
              )}
            </div>
            <div className="flex flex-col gap-2 items-end">
              {provider.npi_verified && (
                <span className="inline-flex items-center gap-1 px-3 py-1 bg-emerald-100 text-emerald-700 text-sm font-medium rounded-full">
                  <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                  </svg>
                  NPI Verified
                </span>
              )}
              {provider.verified && (
                <span className="inline-flex items-center gap-1 px-3 py-1 bg-sky-100 text-sky-700 text-sm font-medium rounded-full">
                  <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                  </svg>
                  Verified Provider
                </span>
              )}
            </div>
          </div>

          {/* Provider Details */}
          <div className="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-3">
            {provider.npi && (
              <div className="bg-gray-50 rounded-lg p-3">
                <p className="text-xs text-gray-500 uppercase tracking-wide">NPI Number</p>
                <p className="font-medium text-gray-900 mt-1" data-testid="provider-npi">{provider.npi}</p>
              </div>
            )}
            {provider.did && (
              <div className="bg-gray-50 rounded-lg p-3">
                <p className="text-xs text-gray-500 uppercase tracking-wide">Provider DID</p>
                <p className="font-mono text-xs text-gray-700 mt-1 break-all" data-testid="provider-did">{provider.did}</p>
              </div>
            )}
          </div>
        </div>

        {/* Active Patient Consents — Feature #102: View consent-scoped data */}
        {/* Feature #313: Patient list navigation for provider portal */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="patient-list-navigation">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900" data-testid="patient-list-heading">
              Patient List
              {activeConsents.length > 0 && (
                <span className="ml-2 px-2 py-0.5 text-sm bg-sky-100 text-sky-700 rounded-full font-medium">
                  {activeConsents.length}
                </span>
              )}
            </h3>
            <button
              onClick={handleRefreshConsents}
              disabled={refreshing}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-sky-700 border border-sky-200 bg-sky-50 rounded-lg hover:bg-sky-100 transition disabled:opacity-50"
              data-testid="refresh-consents-btn"
              aria-label="Refresh access list"
            >
              <svg className={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              {refreshing ? 'Refreshing…' : 'Refresh Access'}
            </button>
          </div>

          <p className="text-sm text-gray-500 mb-4">
            Click "View Patient Data" to see consent-scoped medical information. Only data within the granted scope is accessible.
          </p>

          {activeConsents.length === 0 ? (
            <div className="text-center py-8" data-testid="no-consents">
              <svg className="mx-auto h-10 w-10 text-gray-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <p className="text-gray-500">No active patient consents.</p>
              <p className="text-sm text-gray-400 mt-1">Patients grant you access to their medical records.</p>
            </div>
          ) : (
            <div className="space-y-3" data-testid="patient-list">
              {activeConsents.map((consent) => (
                <div
                  key={consent.id}
                  className="flex items-center justify-between p-4 border border-gray-200 rounded-lg hover:bg-gray-50 cursor-pointer"
                  data-testid={`consent-${consent.id}`}
                >
                  <div>
                    <p className="font-medium text-gray-900" data-testid={`patient-name-${consent.id}`}>
                      {consent.subscriber_name || 'Anonymous Patient'}
                    </p>
                    <div className="flex gap-3 mt-1 items-center flex-wrap">
                      <ScopeBadge scope={consent.scope} />
                      <span className="text-xs text-gray-500">
                        Granted: {new Date(consent.created_at).toLocaleDateString()}
                      </span>
                      {consent.expires_at && (
                        <span className="text-xs text-gray-500">
                          Expires: {new Date(consent.expires_at).toLocaleDateString()}
                        </span>
                      )}
                    </div>
                  </div>
                  <button
                    onClick={() => setViewingConsent(consent)}
                    className="ml-4 px-3 py-1.5 text-sm font-medium bg-sky-600 text-white rounded-lg hover:bg-sky-700 transition whitespace-nowrap"
                    data-testid={`view-patient-data-${consent.id}`}
                  >
                    View Patient Data
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* All Consents History */}
        {consents.length > activeConsents.length && (
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              Consent History ({consents.length - activeConsents.length} inactive)
            </h3>
            <div className="space-y-2">
              {consents.filter(c => activeConsents.indexOf(c) === -1).map((consent) => (
                <div
                  key={consent.id}
                  className="flex items-center justify-between p-3 border border-gray-100 rounded-lg bg-gray-50"
                >
                  <div>
                    <p className="text-sm font-medium text-gray-700">
                      Patient: {consent.subscriber_name || 'Anonymous'}
                    </p>
                    <p className="text-xs text-gray-500 mt-0.5">
                      Granted: {new Date(consent.created_at).toLocaleDateString()}
                      {consent.revoked_at && ` · Revoked: ${new Date(consent.revoked_at).toLocaleDateString()}`}
                    </p>
                  </div>
                  <span className="px-2 py-1 text-xs font-medium bg-gray-200 text-gray-600 rounded-full">
                    {consent.revoked_at ? 'Revoked' : 'Expired'}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Request Additional Access (Feature #103) */}
        {activeConsents.length > 0 && (
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-900">Request Additional Access</h3>
              {!showRequestForm && (
                <button
                  onClick={() => setShowRequestForm(true)}
                  className="px-4 py-2 text-sm font-medium bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 transition"
                  data-testid="request-access-btn"
                >
                  + Request Access
                </button>
              )}
            </div>

            {requestMsg && (
              <div className="mb-4 p-3 bg-emerald-50 border border-emerald-200 text-emerald-700 rounded-lg text-sm">
                {requestMsg}
              </div>
            )}

            {showRequestForm && (
              <form onSubmit={handleRequestAdditionalAccess} className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Patient</label>
                  <select
                    value={requestConsentId}
                    onChange={e => setRequestConsentId(e.target.value)}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500 focus:border-transparent"
                    required
                  >
                    <option value="">Select patient…</option>
                    {activeConsents.map(c => (
                      <option key={c.id} value={c.id}>
                        {c.subscriber_name || 'Anonymous'} (current: {c.scope})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Requested Scope</label>
                  <select
                    value={requestScope}
                    onChange={e => setRequestScope(e.target.value)}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500 focus:border-transparent"
                    required
                  >
                    <option value="">Select scope…</option>
                    {SCOPE_OPTIONS.map(o => (
                      <option key={o.value} value={o.value}>{o.label}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Purpose (optional)</label>
                  <input
                    type="text"
                    value={requestPurpose}
                    onChange={e => setRequestPurpose(e.target.value)}
                    placeholder="e.g. Pre-surgical evaluation"
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500 focus:border-transparent"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Message to Patient (optional)</label>
                  <textarea
                    value={requestMessage}
                    onChange={e => setRequestMessage(e.target.value)}
                    placeholder="Explain why you need this access…"
                    rows={2}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500 focus:border-transparent resize-none"
                  />
                </div>
                {requestError && (
                  <p className="text-red-600 text-sm">{requestError}</p>
                )}
                <div className="flex gap-3">
                  <button
                    type="submit"
                    disabled={requesting}
                    className="px-4 py-2 text-sm font-medium bg-sky-600 text-white rounded-lg hover:bg-sky-700 disabled:opacity-50 transition"
                  >
                    {requesting ? 'Sending…' : 'Send Request'}
                  </button>
                  <button
                    type="button"
                    onClick={() => { setShowRequestForm(false); setRequestError(''); }}
                    className="px-4 py-2 text-sm font-medium text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 transition"
                  >
                    Cancel
                  </button>
                </div>
              </form>
            )}

            {/* Access Requests History */}
            {accessRequests.length > 0 && !showRequestForm && (
              <div className="mt-4">
                <h4 className="text-sm font-medium text-gray-700 mb-2">Recent Access Requests</h4>
                <div className="space-y-2">
                  {accessRequests.slice(0, 5).map(ar => (
                    <div key={ar.id} className="flex items-center justify-between p-3 border border-gray-100 rounded-lg text-sm">
                      <div>
                        <p className="font-medium text-gray-700">{ar.subscriber_name || 'Anonymous'}</p>
                        <p className="text-xs text-gray-500">{ar.requested_scope} · {new Date(ar.requested_at).toLocaleDateString()}</p>
                      </div>
                      <span className={`px-2 py-0.5 text-xs font-medium rounded-full ${
                        ar.status === 'approved' ? 'bg-emerald-100 text-emerald-700' :
                        ar.status === 'denied' ? 'bg-red-100 text-red-700' :
                        'bg-yellow-100 text-yellow-700'
                      }`}>
                        {ar.status}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Clinical Notes Section (Feature #107) */}
        {activeConsents.length > 0 && (
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6" data-testid="clinical-notes-section">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-900">📋 Clinical Notes</h3>
              {!showNoteForm && (
                <button
                  onClick={() => setShowNoteForm(true)}
                  className="px-4 py-2 text-sm font-medium bg-teal-600 text-white rounded-lg hover:bg-teal-700 transition"
                  data-testid="add-clinical-note-btn"
                >
                  + Add Clinical Note
                </button>
              )}
            </div>

            <p className="text-sm text-gray-500 mb-4">
              Add clinical notes to subscriber records. Notes require subscriber approval before being added permanently.
            </p>

            {noteMsg && (
              <div className="mb-4 p-3 bg-teal-50 border border-teal-200 text-teal-700 rounded-lg text-sm" data-testid="note-success-msg">
                {noteMsg}
              </div>
            )}

            {showNoteForm && (
              <form onSubmit={handleAddClinicalNote} className="space-y-4 mb-6 p-4 bg-teal-50 border border-teal-200 rounded-lg" data-testid="clinical-note-form">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Patient</label>
                  <select
                    value={noteConsentId}
                    onChange={e => setNoteConsentId(e.target.value)}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-teal-500"
                    required
                    data-testid="note-patient-select"
                  >
                    <option value="">Select patient…</option>
                    {activeConsents.map(c => (
                      <option key={c.id} value={c.id}>
                        {c.subscriber_name || 'Anonymous'} ({c.scope})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Note Type</label>
                  <select
                    value={noteType}
                    onChange={e => setNoteType(e.target.value)}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-teal-500"
                    data-testid="note-type-select"
                  >
                    <option value="clinical_note">Clinical Note</option>
                    <option value="progress_note">Progress Note</option>
                    <option value="assessment">Assessment</option>
                    <option value="treatment_plan">Treatment Plan</option>
                    <option value="referral_note">Referral Note</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Clinical Note</label>
                  <textarea
                    value={noteText}
                    onChange={e => setNoteText(e.target.value)}
                    placeholder="Enter your clinical observation or note…"
                    rows={4}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-teal-500 resize-none"
                    required
                    data-testid="note-text-area"
                  />
                </div>
                {noteError && (
                  <p className="text-red-600 text-sm" data-testid="note-error">{noteError}</p>
                )}
                <div className="p-3 bg-amber-50 border border-amber-200 rounded-lg text-xs text-amber-700">
                  ⚠️ This note will be sent to the patient for approval before being added to their record.
                </div>
                <div className="flex gap-3">
                  <button
                    type="submit"
                    disabled={submittingNote}
                    className="px-4 py-2 text-sm font-medium bg-teal-600 text-white rounded-lg hover:bg-teal-700 disabled:opacity-50 transition"
                    data-testid="submit-note-btn"
                  >
                    {submittingNote ? 'Submitting…' : 'Submit Note for Approval'}
                  </button>
                  <button
                    type="button"
                    onClick={() => { setShowNoteForm(false); setNoteError(''); setNoteText(''); }}
                    className="px-4 py-2 text-sm font-medium text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 transition"
                  >
                    Cancel
                  </button>
                </div>
              </form>
            )}

            {/* Notes History */}
            {clinicalNotes.length > 0 && !showNoteForm && (
              <div>
                <h4 className="text-sm font-medium text-gray-700 mb-2">Submitted Notes ({clinicalNotes.length})</h4>
                <div className="space-y-2" data-testid="notes-history">
                  {clinicalNotes.slice(0, 10).map(note => (
                    <div key={note.id} className="flex items-start justify-between p-3 border border-gray-100 rounded-lg text-sm" data-testid={`note-${note.id}`}>
                      <div className="flex-1 min-w-0">
                        <p className="font-medium text-gray-800 truncate">{note.subscriber_first_name} {note.subscriber_last_name}</p>
                        <p className="text-xs text-gray-500 mt-0.5 truncate">{note.note_type} · {new Date(note.created_at).toLocaleDateString()}</p>
                        <p className="text-xs text-gray-600 mt-1 line-clamp-2">{note.note_text}</p>
                      </div>
                      <span className={`ml-3 flex-shrink-0 px-2 py-0.5 text-xs font-medium rounded-full ${
                        note.status === 'approved' ? 'bg-emerald-100 text-emerald-700' :
                        note.status === 'rejected' ? 'bg-red-100 text-red-700' :
                        'bg-amber-100 text-amber-700'
                      }`} data-testid={`note-status-${note.id}`}>
                        {note.status === 'pending_approval' ? 'Pending' : note.status}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </main>
    </div>
  );
}

export default ProviderDashboard;
