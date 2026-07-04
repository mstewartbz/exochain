import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import api from '../services/api';

function ProviderRegister() {
  const navigate = useNavigate();
  const [formData, setFormData] = useState({
    email: '',
    password: '',
    confirmPassword: '',
    npi: '',
    facility: '',
    specialty: '',
  });
  const [error, setError] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [success, setSuccess] = useState(null);

  // NPI lookup state
  const [npiLookup, setNpiLookup] = useState(null);
  const [npiLookupLoading, setNpiLookupLoading] = useState(false);
  const [npiLookupError, setNpiLookupError] = useState('');
  const [npiVerified, setNpiVerified] = useState(false);
  const [npiError, setNpiError] = useState('');

  const handleChange = (e) => {
    const newData = { ...formData, [e.target.name]: e.target.value };
    setFormData(newData);

    // Reset NPI verification if NPI changes
    if (e.target.name === 'npi') {
      setNpiLookup(null);
      setNpiVerified(false);
      setNpiLookupError('');
      // Inline NPI validation
      const npiVal = e.target.value;
      if (npiVal === '') {
        setNpiError('');
      } else if (/[^0-9]/.test(npiVal)) {
        setNpiError('NPI must contain only digits');
      } else if (npiVal.length !== 10) {
        setNpiError('NPI must be exactly 10 digits');
      } else {
        setNpiError('');
      }
    }
  };

  const handleNpiLookup = async () => {
    const npiClean = formData.npi.replace(/\D/g, '');
    if (npiClean.length !== 10) {
      setNpiLookupError('NPI must be a 10-digit number');
      return;
    }

    setNpiLookupLoading(true);
    setNpiLookupError('');
    setNpiLookup(null);

    try {
      const res = await api.get('/auth/provider/npi-lookup/' + npiClean);
      setNpiLookup(res.data);
      setNpiVerified(true);

      // Auto-fill facility and specialty from NPI lookup if not already set
      const updates = {};
      if (!formData.facility && res.data.facility) {
        updates.facility = res.data.facility;
      }
      if (!formData.specialty && res.data.taxonomy) {
        updates.specialty = res.data.taxonomy;
      }
      if (Object.keys(updates).length > 0) {
        setFormData(prev => ({ ...prev, ...updates }));
      }
    } catch (err) {
      const errMsg = err.response?.data?.error || 'NPI lookup failed';
      setNpiLookupError(errMsg);
      setNpiVerified(false);
    } finally {
      setNpiLookupLoading(false);
    }
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    setError('');

    if (formData.password.length < 6) {
      setError('Password must be at least 6 characters');
      return;
    }

    if (formData.password !== formData.confirmPassword) {
      setError('Passwords do not match');
      return;
    }

    if (/[^0-9]/.test(formData.npi)) {
      setNpiError('NPI must contain only digits');
      return;
    }
    const npiClean = formData.npi.replace(/\D/g, '');
    if (npiClean.length !== 10) {
      setNpiError('NPI must be exactly 10 digits');
      return;
    }

    setSubmitting(true);
    try {
      const res = await api.post('/auth/provider/register', {
        email: formData.email,
        password: formData.password,
        npi: formData.npi,
        facility: formData.facility,
        specialty: formData.specialty || undefined,
        provider_name: npiLookup ? npiLookup.provider_name : undefined,
        npi_taxonomy: npiLookup ? npiLookup.taxonomy : undefined,
      });

      setSuccess(res.data);
    } catch (err) {
      setError(err.response?.data?.error || 'Registration failed');
    } finally {
      setSubmitting(false);
    }
  };

  if (success) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="max-w-md w-full p-8 bg-white rounded-xl shadow-sm border border-gray-200 text-center">
          <div className="text-emerald-500 text-4xl mb-4">&#9989;</div>
          <h2 className="text-xl font-bold text-gray-900 mb-2">Provider Account Created!</h2>
          <p className="text-gray-600 mb-2">
            Your healthcare provider account has been registered.
          </p>
          <div className="my-4 p-3 bg-sky-50 rounded-lg border border-sky-200 text-left">
            <p className="text-sm text-sky-800">
              <span className="font-medium">Your DID:</span>{' '}
              <code className="text-xs bg-sky-100 px-2 py-0.5 rounded" data-testid="provider-did">{success.user.did}</code>
            </p>
            <p className="text-sm text-sky-800 mt-1">
              <span className="font-medium">NPI:</span>{' '}
              <span data-testid="provider-npi">{success.user.npi}</span>
              {success.user.npi_verified && (
                <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-100 text-emerald-800" data-testid="npi-verified-badge">
                  Verified
                </span>
              )}
            </p>
            {success.user.provider_name && (
              <p className="text-sm text-sky-800 mt-1">
                <span className="font-medium">Provider Name:</span>{' '}
                <span data-testid="provider-name">{success.user.provider_name}</span>
              </p>
            )}
            <p className="text-sm text-sky-800 mt-1">
              <span className="font-medium">Facility:</span>{' '}
              <span data-testid="provider-facility">{success.user.facility}</span>
            </p>
            {success.user.specialty && (
              <p className="text-sm text-sky-800 mt-1">
                <span className="font-medium">Specialty:</span>{' '}
                <span data-testid="provider-specialty">{success.user.specialty}</span>
              </p>
            )}
            {success.user.npi_taxonomy && (
              <p className="text-sm text-sky-800 mt-1">
                <span className="font-medium">NPI Taxonomy:</span>{' '}
                <span data-testid="provider-taxonomy">{success.user.npi_taxonomy}</span>
              </p>
            )}
          </div>
          <Link to="/" className="text-sky-700 hover:text-sky-800 font-medium">
            Go to Home
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12">
      <div className="max-w-md w-full p-8 bg-white rounded-xl shadow-sm border border-gray-200">
        <div className="text-center mb-6">
          <h1 className="text-2xl font-bold text-sky-700">
            LiveSafe<span className="text-emerald-600">.ai</span>
          </h1>
          <h2 className="text-lg font-semibold text-gray-900 mt-4">
            Provider Registration
          </h2>
          <p className="text-gray-600 mt-2">
            Register as a healthcare provider with your NPI number
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Email Address *</label>
            <input
              type="email"
              name="email"
              value={formData.email}
              onChange={handleChange}
              required
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              placeholder="provider@hospital.org"
              data-testid="provider-email-input"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">NPI Number *</label>
            <div className="flex gap-2">
              <input
                type="text"
                name="npi"
                value={formData.npi}
                onChange={handleChange}
                required
                maxLength={10}
                className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                placeholder="1234567893"
                data-testid="provider-npi-input"
              />
              <button
                type="button"
                onClick={handleNpiLookup}
                disabled={npiLookupLoading || formData.npi.replace(/\D/g, '').length !== 10}
                className="px-4 py-2 bg-sky-600 text-white text-sm font-medium rounded-lg hover:bg-sky-700 transition disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
                data-testid="npi-lookup-btn"
              >
                {npiLookupLoading ? 'Looking up...' : 'Verify NPI'}
              </button>
            </div>
            <p className="text-xs text-gray-500 mt-1">10-digit National Provider Identifier</p>

            {/* Inline NPI Validation Error */}
            {npiError && (
              <div className="mt-2 p-2 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" data-testid="npi-format-error">
                {npiError}
              </div>
            )}

            {/* NPI Lookup Error */}
            {npiLookupError && (
              <div className="mt-2 p-2 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" data-testid="npi-lookup-error">
                {npiLookupError}
              </div>
            )}

            {/* NPI Lookup Result */}
            {npiLookup && npiVerified && (
              <div className="mt-2 p-3 bg-emerald-50 border border-emerald-200 rounded-lg" data-testid="npi-lookup-result">
                <div className="flex items-center gap-2 mb-2">
                  <span className="text-emerald-600 font-bold text-sm">&#10003; NPI Verified</span>
                  <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-100 text-emerald-800">
                    {npiLookup.status}
                  </span>
                </div>
                <div className="space-y-1 text-sm text-emerald-900">
                  <p><span className="font-medium">Provider:</span> {npiLookup.provider_name}</p>
                  <p><span className="font-medium">Taxonomy:</span> {npiLookup.taxonomy}</p>
                  <p><span className="font-medium">Facility:</span> {npiLookup.facility}</p>
                  {npiLookup.addresses && npiLookup.addresses[0] && (
                    <p><span className="font-medium">Location:</span> {npiLookup.addresses[0].city}, {npiLookup.addresses[0].state} {npiLookup.addresses[0].postal_code}</p>
                  )}
                  <p><span className="font-medium">Type:</span> {npiLookup.enumeration_type}</p>
                </div>
              </div>
            )}
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Facility Affiliation *</label>
            <input
              type="text"
              name="facility"
              value={formData.facility}
              onChange={handleChange}
              required
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              placeholder="General Hospital"
              data-testid="provider-facility-input"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Specialty</label>
            <select
              name="specialty"
              value={formData.specialty}
              onChange={handleChange}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              data-testid="provider-specialty-select"
            >
              <option value="">Select specialty (optional)</option>
              <option value="Emergency Medicine">Emergency Medicine</option>
              <option value="Internal Medicine">Internal Medicine</option>
              <option value="Family Medicine">Family Medicine</option>
              <option value="Cardiology">Cardiology</option>
              <option value="Neurology">Neurology</option>
              <option value="Orthopedics">Orthopedics</option>
              <option value="Pediatrics">Pediatrics</option>
              <option value="Surgery">Surgery</option>
              <option value="Radiology">Radiology</option>
              <option value="Psychiatry">Psychiatry</option>
              <option value="Other">Other</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Password *</label>
            <input
              type="password"
              name="password"
              value={formData.password}
              onChange={handleChange}
              required
              minLength={6}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              placeholder="Create a password"
              data-testid="provider-password-input"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Confirm Password *</label>
            <input
              type="password"
              name="confirmPassword"
              value={formData.confirmPassword}
              onChange={handleChange}
              required
              minLength={6}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              placeholder="Confirm password"
              data-testid="provider-confirm-password-input"
            />
          </div>

          {error && (
            <div className="p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm">
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={submitting}
            className="w-full py-3 bg-sky-600 text-white font-medium rounded-lg hover:bg-sky-700 transition disabled:opacity-50"
            data-testid="provider-register-btn"
          >
            {submitting ? 'Registering...' : 'Register as Provider'}
          </button>
        </form>

        <p className="mt-4 text-center text-sm text-gray-600">
          Already registered?{' '}
          <Link to="/login" className="text-sky-700 hover:text-sky-800 font-medium">
            Sign in
          </Link>
        </p>
      </div>
    </div>
  );
}

export default ProviderRegister;
