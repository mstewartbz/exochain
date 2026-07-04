import React, { useState, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

const DOCUMENT_TYPES = [
  { value: 'drivers_license', label: "Driver's License" },
  { value: 'passport', label: 'Passport' },
  { value: 'state_id', label: 'State ID' },
  { value: 'military_id', label: 'Military ID' },
  { value: 'national_id', label: 'National ID' },
];

export default function CredentialVault() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const fileInputRef = useRef(null);
  const govIdFileRef = useRef(null);
  const advDirectiveFileRef = useRef(null);

  const [credentials, setCredentials] = useState([]);
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState(0);
  const [showUploadForm, setShowUploadForm] = useState(false);
  const [showGovIdForm, setShowGovIdForm] = useState(false);
  const [showAdvDirectiveForm, setShowAdvDirectiveForm] = useState(false);
  const [extractedFields, setExtractedFields] = useState(null);
  const [editingId, setEditingId] = useState(null);
  const [editForm, setEditForm] = useState({});
  const [message, setMessage] = useState(null);
  const [error, setError] = useState(null);

  // Government ID form state
  const [govIdForm, setGovIdForm] = useState({
    document_type: 'drivers_license',
    issuing_authority: '',
    document_number: '',
  });
  const [govIdUploading, setGovIdUploading] = useState(false);
  const [govIdProgress, setGovIdProgress] = useState(0);

  // Advance Directive form state
  const [advDirectiveForm, setAdvDirectiveForm] = useState({
    title: 'Advance Directive',
    description: '',
    document_date: new Date().toISOString().split('T')[0],
    notary_info: '',
  });
  const [advDirectiveUploading, setAdvDirectiveUploading] = useState(false);
  const [advDirectiveProgress, setAdvDirectiveProgress] = useState(0);

  // POA form state (Feature #119)
  const [showPoaForm, setShowPoaForm] = useState(false);
  const [poaForm, setPoaForm] = useState({
    attorney_name: '',
    attorney_relationship: '',
    document_date: new Date().toISOString().split('T')[0],
    notes: '',
    trustee_id: '',
  });
  const [poaUploading, setPoaUploading] = useState(false);
  const [poaFileRef] = useState(React.createRef());
  const [poaTrustees, setPoaTrustees] = useState([]);

  // Organ donor preference state (Feature #114)
  const [organDonor, setOrganDonor] = useState(false);
  const [organDonorSaving, setOrganDonorSaving] = useState(false);
  const [organDonorMsg, setOrganDonorMsg] = useState(null);
  const [subscriberDid, setSubscriberDid] = useState(null);

  useEffect(() => {
    fetchCredentials();
    fetchProfile();
    fetchTrustees();
    // Check for credential expiry alerts in background (non-blocking)
    api.get('/credentials/expiry-check').catch(() => {});
  }, []);

  async function fetchCredentials() {
    try {
      setLoading(true);
      const res = await api.get('/credentials');
      setCredentials(res.data);
    } catch (err) {
      console.error('Failed to fetch credentials:', err);
      setError('Failed to load credentials');
    } finally {
      setLoading(false);
    }
  }

  // Feature #119: Load trustees for POA linking
  async function fetchTrustees() {
    try {
      const res = await api.get('/subscribers/profile');
      const did = res.data.did;
      if (did) {
        const trusteeRes = await api.get('/pace/trustees/' + did);
        setPoaTrustees(trusteeRes.data || []);
      }
    } catch (err) {
      console.error('Failed to load trustees:', err);
    }
  }

  // Feature #119: Submit POA
  async function handlePoaSubmit(e) {
    e.preventDefault();
    setPoaUploading(true);
    setError(null);
    setMessage(null);
    try {
      const formData = new FormData();
      if (poaForm.attorney_name) formData.append('attorney_name', poaForm.attorney_name);
      if (poaForm.attorney_relationship) formData.append('attorney_relationship', poaForm.attorney_relationship);
      if (poaForm.document_date) formData.append('document_date', poaForm.document_date);
      if (poaForm.notes) formData.append('notes', poaForm.notes);
      if (poaForm.trustee_id) formData.append('trustee_id', poaForm.trustee_id);
      const file = poaFileRef.current && poaFileRef.current.files && poaFileRef.current.files[0];
      if (file) formData.append('poa_document', file);

      const res = await api.post('/credentials/poa', formData, {
        headers: { 'Content-Type': 'multipart/form-data' }
      });
      setMessage('✅ ' + res.data.message);
      setShowPoaForm(false);
      setPoaForm({ attorney_name: '', attorney_relationship: '', document_date: new Date().toISOString().split('T')[0], notes: '', trustee_id: '' });
      fetchCredentials();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to save Power of Attorney');
    } finally {
      setPoaUploading(false);
    }
  }

  // Feature #114: Load subscriber profile for organ donor preference + DID
  async function fetchProfile() {
    try {
      const res = await api.get('/subscribers/profile');
      const profile = res.data;
      setOrganDonor(profile.organ_donor || false);
      setSubscriberDid(profile.did || null);
    } catch (err) {
      console.error('Failed to load profile:', err);
    }
  }

  // Feature #114: Save organ donor preference
  async function handleSaveOrganDonor(newValue) {
    setOrganDonorSaving(true);
    setOrganDonorMsg(null);
    try {
      await api.put('/subscribers/profile', { organ_donor: newValue });
      setOrganDonor(newValue);
      setOrganDonorMsg(newValue
        ? '✅ Organ donor preference saved. Your DID is linked to this preference.'
        : '✅ Organ donor preference updated to No.');
    } catch (err) {
      setOrganDonorMsg('❌ Failed to save organ donor preference: ' + (err.response?.data?.error || err.message));
    } finally {
      setOrganDonorSaving(false);
    }
  }

  async function handleInsuranceUpload(e) {
    e.preventDefault();
    const file = fileInputRef.current?.files?.[0];
    if (!file) {
      setError('Please select an insurance card image to upload');
      return;
    }

    // Validate file format — insurance cards must be image files
    const allowedImageTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'];
    const allowedExtensions = ['jpg', 'jpeg', 'png', 'gif', 'webp'];
    const ext = file.name.toLowerCase().split('.').pop();
    if (!allowedImageTypes.includes(file.type) && !allowedExtensions.includes(ext)) {
      setError(`Unsupported file format ".${ext}". Insurance cards must be images (PNG, JPG, JPEG, GIF, WebP).`);
      if (fileInputRef.current) fileInputRef.current.value = '';
      return;
    }

    setUploading(true);
    setUploadProgress(0);
    setError(null);
    setMessage(null);

    const formData = new FormData();
    formData.append('card_image', file);

    try {
      const res = await api.post('/credentials/insurance', formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
        onUploadProgress: (progressEvent) => {
          const pct = Math.round((progressEvent.loaded * 100) / progressEvent.total);
          setUploadProgress(pct);
        }
      });

      setExtractedFields(res.data.extracted_fields);
      setMessage('Insurance card uploaded and fields extracted successfully!');
      setShowUploadForm(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
      fetchCredentials();
    } catch (err) {
      console.error('Upload error:', err);
      setError(err.response?.data?.error || 'Failed to upload insurance card');
    } finally {
      setUploading(false);
      setUploadProgress(0);
    }
  }

  async function handleGovIdUpload(e) {
    e.preventDefault();
    const file = govIdFileRef.current?.files?.[0];
    if (!file) {
      setError('Please select a government ID document to upload');
      return;
    }

    setGovIdUploading(true);
    setGovIdProgress(0);
    setError(null);
    setMessage(null);

    const formData = new FormData();
    formData.append('id_document', file);
    formData.append('document_type', govIdForm.document_type);
    formData.append('issuing_authority', govIdForm.issuing_authority);
    formData.append('document_number', govIdForm.document_number);

    try {
      const res = await api.post('/credentials/government-id', formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
        onUploadProgress: (progressEvent) => {
          const pct = Math.round((progressEvent.loaded * 100) / progressEvent.total);
          setGovIdProgress(pct);
        }
      });

      setMessage('Government ID uploaded and encrypted successfully!');
      setShowGovIdForm(false);
      setGovIdForm({ document_type: 'drivers_license', issuing_authority: '', document_number: '' });
      if (govIdFileRef.current) govIdFileRef.current.value = '';
      fetchCredentials();
    } catch (err) {
      console.error('Gov ID upload error:', err);
      setError(err.response?.data?.error || 'Failed to upload government ID');
    } finally {
      setGovIdUploading(false);
      setGovIdProgress(0);
    }
  }

  async function handleAdvDirectiveUpload(e) {
    e.preventDefault();
    const file = advDirectiveFileRef.current?.files?.[0];
    if (!file) {
      setError('Please select an advance directive document to upload');
      return;
    }

    setAdvDirectiveUploading(true);
    setAdvDirectiveProgress(0);
    setError(null);
    setMessage(null);

    const formData = new FormData();
    formData.append('directive_document', file);
    formData.append('title', advDirectiveForm.title);
    formData.append('description', advDirectiveForm.description);
    formData.append('document_date', advDirectiveForm.document_date);
    formData.append('notary_info', advDirectiveForm.notary_info);

    try {
      const res = await api.post('/credentials/advance-directive', formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
        onUploadProgress: (progressEvent) => {
          const pct = Math.round((progressEvent.loaded * 100) / progressEvent.total);
          setAdvDirectiveProgress(pct);
        }
      });

      setMessage('Advance directive uploaded and encrypted successfully!');
      setShowAdvDirectiveForm(false);
      setAdvDirectiveForm({ title: 'Advance Directive', description: '', document_date: new Date().toISOString().split('T')[0], notary_info: '' });
      if (advDirectiveFileRef.current) advDirectiveFileRef.current.value = '';
      fetchCredentials();
    } catch (err) {
      console.error('Advance directive upload error:', err);
      setError(err.response?.data?.error || 'Failed to upload advance directive');
    } finally {
      setAdvDirectiveUploading(false);
      setAdvDirectiveProgress(0);
    }
  }

  async function handleDelete(id) {
    if (!confirm('Are you sure you want to delete this credential?')) return;
    try {
      await api.delete(`/credentials/${id}`);
      setMessage('Credential deleted successfully');
      setCredentials(prev => prev.filter(c => c.id !== id));
    } catch (err) {
      setError('Failed to delete credential');
    }
  }

  function startEdit(credential) {
    setEditingId(credential.id);
    setEditForm({
      carrier: credential.carrier || '',
      member_id: credential.member_id || '',
      group_number: credential.group_number || '',
      effective_date: credential.effective_date ? credential.effective_date.split('T')[0] : '',
      expiry_date: credential.expiry_date ? credential.expiry_date.split('T')[0] : '',
    });
  }

  async function handleEditSave(id) {
    try {
      await api.put(`/credentials/${id}`, editForm);
      setMessage('Credential updated successfully');
      setEditingId(null);
      fetchCredentials();
    } catch (err) {
      setError('Failed to update credential');
    }
  }

  // Feature #118: Toggle insurance card emergency visibility
  async function handleToggleInsuranceVisibility(card) {
    const newVisibility = card.visibility === 'emergency_visible' ? 'private' : 'emergency_visible';
    try {
      await api.put(`/credentials/${card.id}/visibility`, { visibility: newVisibility });
      setCredentials(prev => prev.map(c => c.id === card.id ? { ...c, visibility: newVisibility } : c));
      setMessage(
        newVisibility === 'emergency_visible'
          ? '🏥 Insurance card will now be visible to emergency responders during a card scan.'
          : '🔒 Insurance card is now private and will NOT be shown during emergency scans.'
      );
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to update visibility');
    }
  }

  function formatDate(dateStr) {
    if (!dateStr) return 'N/A';
    return new Date(dateStr).toLocaleDateString('en-US', {
      year: 'numeric', month: 'short', day: 'numeric'
    });
  }

  function getGovIdMeta(credential) {
    try {
      return JSON.parse(credential.data_encrypted || '{}');
    } catch {
      return {};
    }
  }

  const [credentialFilter, setCredentialFilter] = useState('all');

  const insuranceCards = credentials.filter(c => c.credential_type === 'insurance_card');
  const governmentIds = credentials.filter(c => c.credential_type === 'government_id');
  const advanceDirectives = credentials.filter(c => c.credential_type === 'advance_directive');
  const powerOfAttorneyDocs = credentials.filter(c => c.credential_type === 'power_of_attorney');

  const CREDENTIAL_FILTERS = [
    { value: 'all', label: 'All', count: credentials.length },
    { value: 'insurance', label: 'Insurance', count: insuranceCards.length },
    { value: 'id', label: 'Government ID', count: governmentIds.length },
    { value: 'directive', label: 'Advance Directive', count: advanceDirectives.length },
    { value: 'poa', label: 'Power of Attorney', count: powerOfAttorneyDocs.length },
  ];

  const showInsurance = credentialFilter === 'all' || credentialFilter === 'insurance';
  const showGovId = credentialFilter === 'all' || credentialFilter === 'id';
  const showDirective = credentialFilter === 'all' || credentialFilter === 'directive';
  const showPoa = credentialFilter === 'all' || credentialFilter === 'poa';

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Messages */}
        {message && (
          <div data-testid="upload-success" className="mb-6 bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg flex justify-between items-center">
            <span>{message}</span>
            <button onClick={() => setMessage(null)} className="text-green-500 hover:text-green-700" aria-label="Dismiss message">&times;</button>
          </div>
        )}
        {error && (
          <div data-testid="upload-error" className="mb-6 bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg flex justify-between items-center">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="text-red-500 hover:text-red-700" aria-label="Dismiss error">&times;</button>
          </div>
        )}

        {/* Extracted Fields Display */}
        {extractedFields && (
          <div className="mb-6 bg-blue-50 border border-blue-200 rounded-lg p-4">
            <h3 className="text-blue-800 font-semibold mb-2">Extracted Insurance Fields</h3>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div><span className="font-medium text-blue-700">Carrier:</span> {extractedFields.carrier}</div>
              <div><span className="font-medium text-blue-700">Member ID:</span> {extractedFields.member_id}</div>
              <div><span className="font-medium text-blue-700">Group Number:</span> {extractedFields.group_number}</div>
              <div><span className="font-medium text-blue-700">Effective Date:</span> {formatDate(extractedFields.effective_date)}</div>
              <div><span className="font-medium text-blue-700">Expiry Date:</span> {formatDate(extractedFields.expiry_date)}</div>
              <div><span className="font-medium text-blue-700">Confidence:</span> {Math.round(extractedFields.confidence * 100)}%</div>
            </div>
            <button
              onClick={() => setExtractedFields(null)}
              className="mt-2 text-sm text-blue-600 hover:text-blue-800"
            >
              Dismiss
            </button>
          </div>
        )}

        {/* Credential Type Filter */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-4 mb-6" data-testid="credential-filter-bar">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-sm font-medium text-gray-600 mr-1">Filter by type:</span>
            {CREDENTIAL_FILTERS.map(f => (
              <button
                key={f.value}
                onClick={() => setCredentialFilter(f.value)}
                data-testid={`filter-btn-${f.value}`}
                className={`px-3 py-1.5 rounded-full text-sm font-medium transition-colors ${
                  credentialFilter === f.value
                    ? 'bg-sky-500 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
              >
                {f.label}
                <span className={`ml-1.5 text-xs px-1.5 py-0.5 rounded-full ${
                  credentialFilter === f.value ? 'bg-sky-400 text-white' : 'bg-gray-200 text-gray-500'
                }`}>
                  {f.count}
                </span>
              </button>
            ))}
          </div>
        </div>

        {/* Government ID Section */}
        {showGovId && <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="gov-id-section">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center space-x-3">
              <div className="w-10 h-10 bg-amber-100 rounded-lg flex items-center justify-center">
                <svg className="w-6 h-6 text-amber-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V8a2 2 0 00-2-2h-5m-4 0V5a2 2 0 114 0v1m-4 0a2 2 0 104 0m-5 8a2 2 0 100-4 2 2 0 000 4zm0 0c1.306 0 2.417.835 2.83 2M9 14a3.001 3.001 0 00-2.83 2M15 11h3m-3 4h2" />
                </svg>
              </div>
              <h2 className="text-xl font-semibold text-gray-900">Government IDs</h2>
            </div>
            <button
              onClick={() => { setShowGovIdForm(!showGovIdForm); setError(null); }}
              className="bg-amber-500 hover:bg-amber-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            >
              {showGovIdForm ? 'Cancel' : 'Add Government ID'}
            </button>
          </div>

          {/* Government ID Upload Form */}
          {showGovIdForm && (
            <form onSubmit={handleGovIdUpload} className="mb-6 bg-amber-50 rounded-lg p-4 border border-amber-200">
              <h3 className="font-medium text-gray-900 mb-2">Upload Government ID</h3>
              <p className="text-sm text-gray-600 mb-4">
                Your document will be encrypted with AES-256-GCM before storage. The raw document is never stored unencrypted and is not accessible via the API.
              </p>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Document Type</label>
                  <select
                    value={govIdForm.document_type}
                    onChange={(e) => setGovIdForm({ ...govIdForm, document_type: e.target.value })}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-amber-500 focus:border-amber-500"
                  >
                    {DOCUMENT_TYPES.map(dt => (
                      <option key={dt.value} value={dt.value}>{dt.label}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Issuing Authority</label>
                  <input
                    type="text"
                    value={govIdForm.issuing_authority}
                    onChange={(e) => setGovIdForm({ ...govIdForm, issuing_authority: e.target.value })}
                    placeholder="e.g., State of California, US Department of State"
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-amber-500 focus:border-amber-500"
                  />
                </div>
              </div>

              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">Document Number (encrypted before storage)</label>
                <input
                  type="password"
                  value={govIdForm.document_number}
                  onChange={(e) => setGovIdForm({ ...govIdForm, document_number: e.target.value })}
                  placeholder="Will be encrypted - only masked version stored"
                  className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-amber-500 focus:border-amber-500"
                />
                <p className="text-xs text-gray-500 mt-1">Only the last 4 characters will be visible. Full number is AES-256-GCM encrypted.</p>
              </div>

              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">ID Document Image/Scan</label>
                <input
                  ref={govIdFileRef}
                  type="file"
                  accept="image/jpeg,image/png,image/gif,image/webp,application/pdf"
                  className="block w-full text-sm text-gray-500
                    file:mr-4 file:py-2 file:px-4
                    file:rounded-lg file:border-0
                    file:text-sm file:font-semibold
                    file:bg-amber-50 file:text-amber-700
                    hover:file:bg-amber-100
                    cursor-pointer"
                />
                <p className="text-xs text-gray-500 mt-1">File will be encrypted immediately after upload. Original file is deleted.</p>
              </div>

              {govIdUploading && (
                <div className="mb-4">
                  <div className="w-full bg-gray-200 rounded-full h-2.5">
                    <div
                      className="bg-amber-500 h-2.5 rounded-full transition-all duration-300"
                      style={{ width: `${govIdProgress}%` }}
                    ></div>
                  </div>
                  <p className="text-sm text-gray-600 mt-1">Encrypting & uploading... {govIdProgress}%</p>
                </div>
              )}

              <button
                type="submit"
                disabled={govIdUploading}
                className="bg-amber-500 hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed text-white px-6 py-2 rounded-lg text-sm font-medium transition-colors"
              >
                {govIdUploading ? 'Encrypting & Uploading...' : 'Upload & Encrypt Document'}
              </button>
            </form>
          )}

          {/* Government IDs List */}
          {loading ? (
            <div className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500 mx-auto mb-2"></div>
              <p className="text-gray-500">Loading credentials...</p>
            </div>
          ) : governmentIds.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              <p className="text-lg mb-1">No government IDs stored yet</p>
              <p className="text-sm">Upload your government-issued ID to securely store it in your encrypted vault.</p>
            </div>
          ) : (
            <div className="space-y-4">
              {governmentIds.map((govId) => {
                const meta = getGovIdMeta(govId);
                return (
                  <div key={govId.id} className="border border-amber-200 rounded-lg p-4 bg-amber-50/30 hover:border-amber-300 transition-colors">
                    <div className="flex items-center justify-between mb-3">
                      <h4 className="font-semibold text-gray-900">{govId.title || 'Government ID'}</h4>
                      <div className="flex items-center space-x-2">
                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                          <svg className="w-3 h-3 mr-1" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z" clipRule="evenodd" />
                          </svg>
                          AES-256-GCM Encrypted
                        </span>
                      </div>
                    </div>
                    <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-sm">
                      <div>
                        <span className="text-gray-500 block text-xs">Document Type</span>
                        <span className="font-medium text-gray-900">
                          {DOCUMENT_TYPES.find(d => d.value === meta.document_type)?.label || meta.document_type || 'N/A'}
                        </span>
                      </div>
                      <div>
                        <span className="text-gray-500 block text-xs">Issuing Authority</span>
                        <span className="font-medium text-gray-900">{meta.issuing_authority || 'N/A'}</span>
                      </div>
                      <div>
                        <span className="text-gray-500 block text-xs">Document Number</span>
                        <span className="font-medium text-gray-900 font-mono">{meta.document_number_masked || '••••••••'}</span>
                      </div>
                      <div>
                        <span className="text-gray-500 block text-xs">Encryption</span>
                        <span className="font-medium text-green-700">{meta.algorithm || 'AES-256-GCM'}</span>
                      </div>
                      <div>
                        <span className="text-gray-500 block text-xs">Upload Date</span>
                        <span className="font-medium text-gray-900">{formatDate(meta.upload_date || govId.created_at)}</span>
                      </div>
                      <div>
                        <span className="text-gray-500 block text-xs">Visibility</span>
                        <span className="font-medium text-gray-900 capitalize">{govId.visibility || 'private'}</span>
                      </div>
                    </div>
                    <div className="mt-3 flex items-center justify-between">
                      <div className="text-xs text-gray-400">
                        Raw document data is never exposed. Only encrypted metadata is stored.
                      </div>
                      <button
                        onClick={() => handleDelete(govId.id)}
                        className="text-red-600 hover:text-red-700 text-sm font-medium"
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>}

        {/* Advance Directive Section */}
        {showDirective && <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="directive-section">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center space-x-3">
              <div className="w-10 h-10 bg-purple-100 rounded-lg flex items-center justify-center">
                <svg className="w-6 h-6 text-purple-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
              </div>
              <div>
                <h2 className="text-xl font-semibold text-gray-900">Advance Directives</h2>
                <p className="text-xs text-purple-600 font-medium">Encrypted LiveSafe storage</p>
              </div>
            </div>
            <button
              onClick={() => { setShowAdvDirectiveForm(!showAdvDirectiveForm); setError(null); }}
              className="bg-purple-500 hover:bg-purple-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            >
              {showAdvDirectiveForm ? 'Cancel' : 'Upload Advance Directive'}
            </button>
          </div>

          {/* Advance Directive Upload Form */}
          {showAdvDirectiveForm && (
            <form onSubmit={handleAdvDirectiveUpload} className="mb-6 bg-purple-50 rounded-lg p-4 border border-purple-200">
              <h3 className="font-medium text-gray-900 mb-2">Upload Advance Directive / Living Will</h3>
              <p className="text-sm text-gray-600 mb-4">
                Your advance directive will be encrypted with AES-256-GCM and stored in LiveSafe. EXOCHAIN anchoring remains pending adapter verification.
              </p>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Document Title</label>
                  <input
                    type="text"
                    value={advDirectiveForm.title}
                    onChange={(e) => setAdvDirectiveForm({ ...advDirectiveForm, title: e.target.value })}
                    placeholder="e.g., Advance Directive, Living Will, Healthcare Proxy"
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-purple-500 focus:border-purple-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Document Date</label>
                  <input
                    type="date"
                    value={advDirectiveForm.document_date}
                    onChange={(e) => setAdvDirectiveForm({ ...advDirectiveForm, document_date: e.target.value })}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-purple-500 focus:border-purple-500"
                  />
                </div>
              </div>

              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">Description (optional)</label>
                <textarea
                  value={advDirectiveForm.description}
                  onChange={(e) => setAdvDirectiveForm({ ...advDirectiveForm, description: e.target.value })}
                  rows={2}
                  placeholder="Brief description of the document contents"
                  className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-purple-500 focus:border-purple-500"
                />
              </div>

              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">Notary / Attorney Info (optional)</label>
                <input
                  type="text"
                  value={advDirectiveForm.notary_info}
                  onChange={(e) => setAdvDirectiveForm({ ...advDirectiveForm, notary_info: e.target.value })}
                  placeholder="e.g., Notarized by John Smith, State of California"
                  className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-purple-500 focus:border-purple-500"
                />
              </div>

              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">Document File</label>
                <input
                  ref={advDirectiveFileRef}
                  type="file"
                  accept="image/jpeg,image/png,image/gif,image/webp,application/pdf"
                  className="block w-full text-sm text-gray-500
                    file:mr-4 file:py-2 file:px-4
                    file:rounded-lg file:border-0
                    file:text-sm file:font-semibold
                    file:bg-purple-50 file:text-purple-700
                    hover:file:bg-purple-100
                    cursor-pointer"
                />
                <p className="text-xs text-gray-500 mt-1">Accepted: PDF, JPEG, PNG. File will be AES-256-GCM encrypted and linked to your LiveSafe DID.</p>
              </div>

              {advDirectiveUploading && (
                <div className="mb-4">
                  <div className="w-full bg-gray-200 rounded-full h-2.5">
                    <div
                      className="bg-purple-500 h-2.5 rounded-full transition-all duration-300"
                      style={{ width: `${advDirectiveProgress}%` }}
                    ></div>
                  </div>
                  <p className="text-sm text-gray-600 mt-1">Encrypting upload... {advDirectiveProgress}%</p>
                </div>
              )}

              <button
                type="submit"
                disabled={advDirectiveUploading}
                className="bg-purple-500 hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed text-white px-6 py-2 rounded-lg text-sm font-medium transition-colors"
              >
                {advDirectiveUploading ? 'Encrypting upload...' : 'Upload & Encrypt'}
              </button>
            </form>
          )}

          {/* Advance Directives List */}
          {loading ? (
            <div className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-500 mx-auto mb-2"></div>
              <p className="text-gray-500">Loading credentials...</p>
            </div>
          ) : advanceDirectives.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              <p className="text-lg mb-1">No advance directives stored yet</p>
              <p className="text-sm">Upload your advance directive or living will to store it in encrypted LiveSafe storage linked to your DID.</p>
            </div>
          ) : (
            <div className="space-y-4">
              {advanceDirectives.map((directive) => {
                let meta = {};
                try { meta = JSON.parse(directive.data_encrypted || '{}'); } catch (e) {}
                let receipt = {};
                try { receipt = JSON.parse(directive.exochain_receipt || '{}'); } catch (e) {}
                return (
                  <div key={directive.id} className="border border-purple-200 rounded-lg p-4 bg-purple-50/30 hover:border-purple-300 transition-colors">
                    <div className="flex items-center justify-between mb-3">
                      <h4 className="font-semibold text-gray-900">{directive.title || 'Advance Directive'}</h4>
                      <div className="flex items-center space-x-2">
                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-purple-100 text-purple-800">
                          <svg className="w-3 h-3 mr-1" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z" clipRule="evenodd" />
                          </svg>
                          Encrypted
                        </span>
                      </div>
                    </div>
                    <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-sm mb-3">
                      <div>
                        <span className="text-gray-500 block text-xs">Document Date</span>
                        <span className="font-medium text-gray-900">{formatDate(meta.document_date)}</span>
                      </div>
                      <div>
                        <span className="text-gray-500 block text-xs">Upload Date</span>
                        <span className="font-medium text-gray-900">{formatDate(meta.upload_date || directive.created_at)}</span>
                      </div>
                      <div>
                        <span className="text-gray-500 block text-xs">Encryption</span>
                        <span className="font-medium text-green-700">{meta.algorithm || 'AES-256-GCM'}</span>
                      </div>
                      {meta.description && (
                        <div className="col-span-2">
                          <span className="text-gray-500 block text-xs">Description</span>
                          <span className="font-medium text-gray-900">{meta.description}</span>
                        </div>
                      )}
                      {meta.notary_info && (
                        <div className="col-span-2">
                          <span className="text-gray-500 block text-xs">Notary / Attorney</span>
                          <span className="font-medium text-gray-900">{meta.notary_info}</span>
                        </div>
                      )}
                    </div>
                    {receipt.receipt_id && (
                      <div className="bg-purple-50 border border-purple-200 rounded p-2 mb-3">
                        <div className="text-xs text-purple-700 font-medium mb-1">Local custody receipt</div>
                        <div className="text-xs text-gray-600 font-mono break-all">{receipt.receipt_id}</div>
                        <div className="text-xs text-gray-500 mt-1">
                          Depositor DID: <span className="font-mono">{receipt.depositor_did}</span>
                        </div>
                        <div className="text-xs text-gray-500">
                          Status: <span className="text-green-600 font-medium">{receipt.bailment_status || 'active'}</span>
                        </div>
                      </div>
                    )}
                    {meta.subscriber_did && (
                      <div className="text-xs text-gray-400 mb-2">
                        Linked DID: <span className="font-mono">{meta.subscriber_did}</span>
                      </div>
                    )}
                    <div className="flex items-center justify-between">
                      <div className="text-xs text-gray-400">
                        Encrypted file stored securely. Raw document never exposed via API.
                      </div>
                      <button
                        onClick={() => handleDelete(directive.id)}
                        className="text-red-600 hover:text-red-700 text-sm font-medium"
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>}

        {/* Insurance Cards Section */}
        {showInsurance && <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="insurance-section">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold text-gray-900">Insurance Cards</h2>
            <button
              onClick={() => { setShowUploadForm(!showUploadForm); setError(null); }}
              className="bg-sky-500 hover:bg-sky-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            >
              {showUploadForm ? 'Cancel' : 'Upload Insurance Card'}
            </button>
          </div>

          {/* Upload Form */}
          {showUploadForm && (
            <form onSubmit={handleInsuranceUpload} className="mb-6 bg-gray-50 rounded-lg p-4 border border-gray-200">
              <h3 className="font-medium text-gray-900 mb-3">Upload Insurance Card Image</h3>
              <p className="text-sm text-gray-600 mb-3">
                Upload a photo or scan of your insurance card. The system will automatically extract carrier, member ID, group number, and effective dates.
              </p>
              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">Card Image</label>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/jpeg,image/png,image/gif,image/webp"
                  data-testid="insurance-card-file-input"
                  className="block w-full text-sm text-gray-500
                    file:mr-4 file:py-2 file:px-4
                    file:rounded-lg file:border-0
                    file:text-sm file:font-semibold
                    file:bg-sky-50 file:text-sky-700
                    hover:file:bg-sky-100
                    cursor-pointer"
                />
                <p className="text-xs text-gray-500 mt-1">Accepted formats: PNG, JPG, JPEG, GIF, WebP</p>
              </div>

              {uploading && (
                <div className="mb-4">
                  <div className="w-full bg-gray-200 rounded-full h-2.5">
                    <div
                      className="bg-sky-500 h-2.5 rounded-full transition-all duration-300"
                      style={{ width: `${uploadProgress}%` }}
                    ></div>
                  </div>
                  <p className="text-sm text-gray-600 mt-1">Uploading... {uploadProgress}%</p>
                </div>
              )}

              <button
                type="submit"
                disabled={uploading}
                className="bg-sky-500 hover:bg-sky-600 disabled:opacity-50 disabled:cursor-not-allowed text-white px-6 py-2 rounded-lg text-sm font-medium transition-colors"
              >
                {uploading ? 'Uploading & Extracting...' : 'Upload & Extract Fields'}
              </button>
            </form>
          )}

          {/* Insurance Cards List */}
          {loading ? (
            <div className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500 mx-auto mb-2"></div>
              <p className="text-gray-500">Loading credentials...</p>
            </div>
          ) : insuranceCards.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              <p className="text-lg mb-1">No insurance cards uploaded yet</p>
              <p className="text-sm">Upload your insurance card to securely store your coverage details.</p>
            </div>
          ) : (
            <div className="space-y-4">
              {insuranceCards.map((card) => (
                <div key={card.id} className="border border-gray-200 rounded-lg p-4 hover:border-sky-200 transition-colors">
                  {editingId === card.id ? (
                    /* Edit Mode */
                    <div className="space-y-3">
                      <h4 className="font-medium text-gray-900 mb-2">Edit Insurance Card</h4>
                      <div className="grid grid-cols-2 gap-3">
                        <div>
                          <label className="block text-xs font-medium text-gray-500 mb-1">Carrier</label>
                          <input
                            type="text"
                            value={editForm.carrier}
                            onChange={(e) => setEditForm({ ...editForm, carrier: e.target.value })}
                            className="w-full border border-gray-300 rounded-lg px-3 py-1.5 text-sm focus:ring-sky-500 focus:border-sky-500"
                          />
                        </div>
                        <div>
                          <label className="block text-xs font-medium text-gray-500 mb-1">Member ID</label>
                          <input
                            type="text"
                            value={editForm.member_id}
                            onChange={(e) => setEditForm({ ...editForm, member_id: e.target.value })}
                            className="w-full border border-gray-300 rounded-lg px-3 py-1.5 text-sm focus:ring-sky-500 focus:border-sky-500"
                          />
                        </div>
                        <div>
                          <label className="block text-xs font-medium text-gray-500 mb-1">Group Number</label>
                          <input
                            type="text"
                            value={editForm.group_number}
                            onChange={(e) => setEditForm({ ...editForm, group_number: e.target.value })}
                            className="w-full border border-gray-300 rounded-lg px-3 py-1.5 text-sm focus:ring-sky-500 focus:border-sky-500"
                          />
                        </div>
                        <div>
                          <label className="block text-xs font-medium text-gray-500 mb-1">Effective Date</label>
                          <input
                            type="date"
                            value={editForm.effective_date}
                            onChange={(e) => setEditForm({ ...editForm, effective_date: e.target.value })}
                            className="w-full border border-gray-300 rounded-lg px-3 py-1.5 text-sm focus:ring-sky-500 focus:border-sky-500"
                          />
                        </div>
                        <div>
                          <label className="block text-xs font-medium text-gray-500 mb-1">Expiry Date</label>
                          <input
                            type="date"
                            value={editForm.expiry_date}
                            onChange={(e) => setEditForm({ ...editForm, expiry_date: e.target.value })}
                            className="w-full border border-gray-300 rounded-lg px-3 py-1.5 text-sm focus:ring-sky-500 focus:border-sky-500"
                          />
                        </div>
                      </div>
                      <div className="flex space-x-2">
                        <button
                          onClick={() => handleEditSave(card.id)}
                          className="bg-green-500 hover:bg-green-600 text-white px-4 py-1.5 rounded-lg text-sm font-medium"
                        >
                          Save Changes
                        </button>
                        <button
                          onClick={() => setEditingId(null)}
                          className="bg-gray-200 hover:bg-gray-300 text-gray-700 px-4 py-1.5 rounded-lg text-sm font-medium"
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  ) : (
                    /* View Mode */
                    <div>
                      <div className="flex items-center justify-between mb-2">
                        <h4 className="font-semibold text-gray-900">{card.title || 'Insurance Card'}</h4>
                        <div className="flex items-center space-x-2">
                          <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                            Stored Securely
                          </span>
                        </div>
                      </div>
                      <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-sm">
                        <div>
                          <span className="text-gray-500 block text-xs">Carrier</span>
                          <span className="font-medium text-gray-900">{card.carrier || 'N/A'}</span>
                        </div>
                        <div>
                          <span className="text-gray-500 block text-xs">Member ID</span>
                          <span className="font-medium text-gray-900">{card.member_id || 'N/A'}</span>
                        </div>
                        <div>
                          <span className="text-gray-500 block text-xs">Group Number</span>
                          <span className="font-medium text-gray-900">{card.group_number || 'N/A'}</span>
                        </div>
                        <div>
                          <span className="text-gray-500 block text-xs">Effective Date</span>
                          <span className="font-medium text-gray-900">{formatDate(card.effective_date)}</span>
                        </div>
                        <div>
                          <span className="text-gray-500 block text-xs">Expiry Date</span>
                          <span className="font-medium text-gray-900">{formatDate(card.expiry_date)}</span>
                        </div>
                        <div>
                          <span className="text-gray-500 block text-xs">Visibility</span>
                          <span className="font-medium text-gray-900 capitalize">{card.visibility || 'private'}</span>
                        </div>
                      </div>
                      {/* Feature #118: Emergency visibility toggle */}
                      <div className="mt-3 p-3 rounded-lg bg-gray-50 border border-gray-200">
                        <div className="flex items-center justify-between">
                          <div>
                            <span className="text-sm font-medium text-gray-700">🏥 Emergency Responder Access</span>
                            <p className="text-xs text-gray-500 mt-0.5">
                              {card.visibility === 'emergency_visible'
                                ? 'Visible to ER responders during emergency card scan'
                                : 'Hidden from emergency responders (private)'}
                            </p>
                          </div>
                          <button
                            onClick={() => handleToggleInsuranceVisibility(card)}
                            data-testid={`insurance-visibility-toggle-${card.id}`}
                            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-sky-500 ${
                              card.visibility === 'emergency_visible'
                                ? 'bg-green-500'
                                : 'bg-gray-300'
                            }`}
                            title={card.visibility === 'emergency_visible' ? 'Click to hide from ER' : 'Click to show to ER during scan'}
                          >
                            <span
                              className={`inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform ${
                                card.visibility === 'emergency_visible' ? 'translate-x-6' : 'translate-x-1'
                              }`}
                            />
                          </button>
                        </div>
                        {card.visibility === 'emergency_visible' && (
                          <p className="text-xs text-green-700 mt-1 font-medium">
                            ✅ Carrier, Member ID, and Group Number will be shown to responders
                          </p>
                        )}
                      </div>
                      <div className="mt-2 flex space-x-2">
                        <button
                          onClick={() => startEdit(card)}
                          className="text-sky-700 hover:text-sky-800 text-sm font-medium"
                        >
                          Edit
                        </button>
                        <button
                          onClick={() => handleDelete(card.id)}
                          className="text-red-600 hover:text-red-700 text-sm font-medium"
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>}

        {/* Feature #119: Power of Attorney */}
        {showPoa && <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="poa-section">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center space-x-3">
              <div className="w-10 h-10 bg-indigo-100 rounded-lg flex items-center justify-center">
                <span className="text-xl">⚖️</span>
              </div>
              <div>
                <h2 className="text-xl font-semibold text-gray-900">Power of Attorney</h2>
                <p className="text-sm text-gray-500">Designate a legal representative linked to your PACE trustee.</p>
              </div>
            </div>
            <button
              onClick={() => { setShowPoaForm(!showPoaForm); setError(null); }}
              data-testid="add-poa-btn"
              className="bg-indigo-500 hover:bg-indigo-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            >
              {showPoaForm ? 'Cancel' : '+ Add POA'}
            </button>
          </div>

          {showPoaForm && (
            <form onSubmit={handlePoaSubmit} className="mb-6 bg-indigo-50 rounded-lg p-4 border border-indigo-200" data-testid="poa-form">
              <h3 className="font-medium text-gray-900 mb-3">Add Power of Attorney Designation</h3>
              <p className="text-sm text-gray-600 mb-4">
                Store your POA designation in encrypted LiveSafe storage linked to your DID. Optionally link to a PACE trustee.
              </p>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Attorney Name</label>
                  <input
                    type="text"
                    value={poaForm.attorney_name}
                    onChange={e => setPoaForm({ ...poaForm, attorney_name: e.target.value })}
                    placeholder="Full legal name of attorney-in-fact"
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-indigo-500 focus:border-indigo-500"
                    data-testid="poa-attorney-name"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Relationship</label>
                  <input
                    type="text"
                    value={poaForm.attorney_relationship}
                    onChange={e => setPoaForm({ ...poaForm, attorney_relationship: e.target.value })}
                    placeholder="e.g., Spouse, Sibling, Friend"
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-indigo-500 focus:border-indigo-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Document Date</label>
                  <input
                    type="date"
                    value={poaForm.document_date}
                    onChange={e => setPoaForm({ ...poaForm, document_date: e.target.value })}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-indigo-500 focus:border-indigo-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Link to PACE Trustee</label>
                  <select
                    value={poaForm.trustee_id}
                    onChange={e => setPoaForm({ ...poaForm, trustee_id: e.target.value })}
                    className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-indigo-500 focus:border-indigo-500"
                    data-testid="poa-trustee-select"
                  >
                    <option value="">— No PACE trustee link —</option>
                    {poaTrustees.map(t => (
                      <option key={t.id} value={t.id}>
                        {t.first_name || t.last_name ? (t.first_name + ' ' + t.last_name).trim() : t.email} ({t.role})
                      </option>
                    ))}
                  </select>
                </div>
              </div>
              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">Notes</label>
                <textarea
                  value={poaForm.notes}
                  onChange={e => setPoaForm({ ...poaForm, notes: e.target.value })}
                  placeholder="Any additional notes about this POA"
                  rows={2}
                  className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-indigo-500 focus:border-indigo-500"
                />
              </div>
              <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">POA Document (Optional)</label>
                <input
                  ref={poaFileRef}
                  type="file"
                  accept="application/pdf,image/jpeg,image/png"
                  data-testid="poa-file-input"
                  className="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:bg-indigo-50 file:text-indigo-700 hover:file:bg-indigo-100 cursor-pointer"
                />
                <p className="text-xs text-gray-500 mt-1">PDF or image. Will be AES-256-GCM encrypted and stored in LiveSafe.</p>
              </div>
              <button
                type="submit"
                disabled={poaUploading}
                data-testid="poa-submit-btn"
                className="bg-indigo-500 hover:bg-indigo-600 disabled:opacity-50 text-white px-6 py-2 rounded-lg text-sm font-medium transition-colors"
              >
                {poaUploading ? 'Saving...' : 'Save Power of Attorney'}
              </button>
            </form>
          )}

          {powerOfAttorneyDocs.length === 0 && !showPoaForm ? (
            <div className="text-center py-8 text-gray-500">
              <p className="text-lg mb-1">No POA designations stored yet</p>
              <p className="text-sm">Add your Power of Attorney to link it to your DID and PACE trustee structure.</p>
            </div>
          ) : (
            <div className="space-y-4">
              {powerOfAttorneyDocs.map(poa => {
                let meta = {};
                try { meta = JSON.parse(poa.data_encrypted); } catch(e) {}
                let receipt = {};
                try { receipt = JSON.parse(poa.exochain_receipt); } catch(e) {}
                return (
                  <div key={poa.id} className="border border-indigo-200 rounded-lg p-4 bg-indigo-50/30" data-testid={`poa-item-${poa.id}`}>
                    <div className="flex items-center justify-between mb-3">
                      <h4 className="font-semibold text-gray-900">{poa.title}</h4>
                      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-indigo-100 text-indigo-800">
                        Encrypted POA
                      </span>
                    </div>
                    <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-sm mb-3">
                      {meta.attorney_name && (
                        <div>
                          <span className="text-gray-500 block text-xs">Attorney-in-Fact</span>
                          <span className="font-medium text-gray-900">{meta.attorney_name}</span>
                        </div>
                      )}
                      {meta.attorney_relationship && (
                        <div>
                          <span className="text-gray-500 block text-xs">Relationship</span>
                          <span className="font-medium text-gray-900">{meta.attorney_relationship}</span>
                        </div>
                      )}
                      {meta.document_date && (
                        <div>
                          <span className="text-gray-500 block text-xs">Document Date</span>
                          <span className="font-medium text-gray-900">{formatDate(meta.document_date)}</span>
                        </div>
                      )}
                    </div>
                    {/* PACE Trustee Mapping */}
                    {meta.pace_trustee_did && (
                      <div className="bg-blue-50 border border-blue-200 rounded p-2 mb-3" data-testid={`poa-trustee-${poa.id}`}>
                        <div className="text-xs text-blue-700 font-medium mb-1">🔗 Linked PACE Trustee</div>
                        <div className="text-xs text-gray-700">
                          {meta.pace_trustee_name || meta.pace_trustee_email} — <span className="capitalize">{meta.pace_trustee_role}</span>
                        </div>
                        <div className="text-xs text-gray-500 font-mono break-all mt-0.5">{meta.pace_trustee_did}</div>
                      </div>
                    )}
                    {/* Local custody receipt */}
                    {receipt.receipt_id && (
                      <div className="bg-indigo-50 border border-indigo-200 rounded p-2 mb-3">
                        <div className="text-xs text-indigo-700 font-medium mb-1">Local custody receipt</div>
                        <div className="text-xs text-gray-600 font-mono break-all">{receipt.receipt_id}</div>
                        <div className="text-xs text-gray-500 mt-0.5">
                          Status: <span className="text-green-600 font-medium">{receipt.bailment_status || 'active'}</span>
                        </div>
                      </div>
                    )}
                    {meta.subscriber_did && (
                      <div className="text-xs text-gray-400 mb-2">
                        Linked DID: <span className="font-mono">{meta.subscriber_did}</span>
                      </div>
                    )}
                    <div className="flex items-center justify-between">
                      <span className="text-xs text-gray-400">
                        {meta.has_document ? '📄 Document uploaded & encrypted' : '📋 Metadata only'}
                      </span>
                      <button
                        onClick={() => handleDelete(poa.id)}
                        className="text-red-600 hover:text-red-700 text-sm font-medium"
                        data-testid={`poa-delete-${poa.id}`}
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>}

        {/* Feature #114: Organ Donor Preference */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="organ-donor-section">
          <div className="flex items-center space-x-3 mb-4">
            <div className="w-10 h-10 bg-red-100 rounded-lg flex items-center justify-center">
              <span className="text-xl">❤️</span>
            </div>
            <div>
              <h2 className="text-xl font-semibold text-gray-900">Organ Donor Preference</h2>
              <p className="text-sm text-gray-500">Your preference is linked to your subscriber DID and stored securely.</p>
            </div>
          </div>

          {organDonorMsg && (
            <div className={`mb-4 px-4 py-3 rounded-lg text-sm ${organDonorMsg.startsWith('✅') ? 'bg-green-50 border border-green-200 text-green-700' : 'bg-red-50 border border-red-200 text-red-700'}`}>
              {organDonorMsg}
            </div>
          )}

          <div className="bg-gray-50 rounded-lg p-4 border border-gray-200">
            <div className="flex items-center justify-between mb-3">
              <div>
                <p className="text-sm font-medium text-gray-900">Current Status</p>
                <p className="text-xs text-gray-500 mt-0.5">
                  {organDonor
                    ? '✅ You are registered as an organ donor'
                    : '⭕ You have not registered as an organ donor'}
                </p>
              </div>
              <span data-testid="organ-donor-status" className={`px-3 py-1 rounded-full text-sm font-semibold ${organDonor ? 'bg-green-100 text-green-800' : 'bg-gray-200 text-gray-700'}`}>
                {organDonor ? 'Yes' : 'No'}
              </span>
            </div>

            <div className="flex gap-3">
              <button
                onClick={() => handleSaveOrganDonor(true)}
                disabled={organDonorSaving || organDonor === true}
                data-testid="organ-donor-yes-btn"
                className={`flex-1 py-2 px-4 rounded-lg text-sm font-medium transition-colors ${
                  organDonor === true
                    ? 'bg-green-500 text-white cursor-default'
                    : 'bg-gray-100 text-gray-700 hover:bg-green-100 hover:text-green-800 border border-gray-300'
                } disabled:opacity-60`}
              >
                {organDonorSaving && organDonor !== true ? '...' : '✅ Yes, I am an organ donor'}
              </button>
              <button
                onClick={() => handleSaveOrganDonor(false)}
                disabled={organDonorSaving || organDonor === false}
                data-testid="organ-donor-no-btn"
                className={`flex-1 py-2 px-4 rounded-lg text-sm font-medium transition-colors ${
                  organDonor === false
                    ? 'bg-gray-400 text-white cursor-default'
                    : 'bg-gray-100 text-gray-700 hover:bg-red-100 hover:text-red-800 border border-gray-300'
                } disabled:opacity-60`}
              >
                {organDonorSaving && organDonor !== false ? '...' : '⭕ No, I am not an organ donor'}
              </button>
            </div>

            {subscriberDid && (
              <div className="mt-4 bg-blue-50 border border-blue-200 rounded p-3" data-testid="organ-donor-did">
                <p className="text-xs text-blue-700 font-medium mb-1">🔗 Linked to Subscriber DID</p>
                <p className="text-xs text-gray-600 font-mono break-all">{subscriberDid}</p>
                <p className="text-xs text-gray-500 mt-1">
                  Your organ donor preference is permanently associated with your decentralized identity.
                </p>
              </div>
            )}
          </div>
        </div>

        {/* All Credentials Summary */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">All Credentials</h2>
          {credentials.length === 0 ? (
            <p className="text-gray-500 text-center py-4">No credentials stored yet.</p>
          ) : (
            <div className="text-sm text-gray-600 space-y-1">
              <p>Total credentials: <span className="font-semibold">{credentials.length}</span></p>
              <p>Government IDs: <span className="font-semibold">{governmentIds.length}</span></p>
              <p>Insurance cards: <span className="font-semibold">{insuranceCards.length}</span></p>
              <p>Advance Directives <span className="text-purple-600">(encrypted)</span>: <span className="font-semibold">{advanceDirectives.length}</span></p>
              <p>Power of Attorney <span className="text-indigo-600">(encrypted)</span>: <span className="font-semibold">{powerOfAttorneyDocs.length}</span></p>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
