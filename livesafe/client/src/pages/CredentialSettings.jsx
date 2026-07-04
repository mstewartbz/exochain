import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

const VISIBILITY_OPTIONS = [
  { value: 'private', label: 'Private', description: 'Only you can see this credential', color: 'gray' },
  { value: 'emergency_visible', label: 'Emergency Visible', description: 'Visible to first responders during emergency scans', color: 'amber' },
  { value: 'always_visible', label: 'Always Visible', description: 'Visible to any authorized provider with access', color: 'green' },
];

const CREDENTIAL_TYPE_LABELS = {
  insurance_card: 'Insurance Cards',
  government_id: 'Government IDs',
  advance_directive: 'Advance Directives',
};

const CREDENTIAL_TYPE_DESCRIPTIONS = {
  insurance_card: 'Your health insurance coverage information',
  government_id: 'Government-issued identification documents',
  advance_directive: 'Advance healthcare directives and living wills',
};

const CREDENTIAL_TYPE_ICONS = {
  insurance_card: (
    <svg className="w-5 h-5 text-sky-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z" />
    </svg>
  ),
  government_id: (
    <svg className="w-5 h-5 text-amber-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V8a2 2 0 00-2-2h-5m-4 0V5a2 2 0 114 0v1m-4 0a2 2 0 104 0m-5 8a2 2 0 100-4 2 2 0 000 4zm0 0c1.306 0 2.417.835 2.83 2M9 14a3.001 3.001 0 00-2.83 2M15 11h3m-3 4h2" />
    </svg>
  ),
  advance_directive: (
    <svg className="w-5 h-5 text-purple-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
    </svg>
  ),
};

function VisibilityBadge({ visibility }) {
  if (visibility === 'emergency_visible') {
    return (
      <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-amber-100 text-amber-800">
        Emergency Visible
      </span>
    );
  }
  if (visibility === 'always_visible') {
    return (
      <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
        Always Visible
      </span>
    );
  }
  return (
    <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-700">
      Private
    </span>
  );
}

export default function CredentialSettings() {
  const { user } = useAuth();
  const navigate = useNavigate();

  const [credentials, setCredentials] = useState([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState({});
  const [message, setMessage] = useState(null);
  const [error, setError] = useState(null);

  // Type-level visibility settings (for bulk update)
  const [typeSettings, setTypeSettings] = useState({
    insurance_card: 'private',
    government_id: 'private',
    advance_directive: 'private',
  });
  const [typeApplying, setTypeApplying] = useState({});

  useEffect(() => {
    fetchCredentials();
  }, []);

  async function fetchCredentials() {
    try {
      setLoading(true);
      const res = await api.get('/credentials');
      setCredentials(res.data);

      // Infer default type settings from existing credentials
      const settings = { insurance_card: 'private', government_id: 'private', advance_directive: 'private' };
      res.data.forEach(function(c) {
        if (c.credential_type in settings && c.visibility) {
          settings[c.credential_type] = c.visibility;
        }
      });
      setTypeSettings(settings);
    } catch (err) {
      console.error('Failed to fetch credentials:', err);
      setError('Failed to load credentials');
    } finally {
      setLoading(false);
    }
  }

  async function handleVisibilityChange(credentialId, newVisibility) {
    setSaving(prev => ({ ...prev, [credentialId]: true }));
    setError(null);
    try {
      await api.put('/credentials/' + credentialId + '/visibility', { visibility: newVisibility });
      setCredentials(prev => prev.map(c =>
        c.id === credentialId ? { ...c, visibility: newVisibility } : c
      ));
      setMessage('Access setting updated successfully');
      setTimeout(() => setMessage(null), 3000);
    } catch (err) {
      console.error('Failed to update visibility:', err);
      setError(err.response?.data?.error || 'Failed to update access setting');
    } finally {
      setSaving(prev => ({ ...prev, [credentialId]: false }));
    }
  }

  async function handleApplyToType(credentialType) {
    const newVisibility = typeSettings[credentialType];
    const typeCredentials = credentials.filter(c => c.credential_type === credentialType);

    if (typeCredentials.length === 0) {
      setMessage('No ' + CREDENTIAL_TYPE_LABELS[credentialType] + ' to update');
      setTimeout(() => setMessage(null), 3000);
      return;
    }

    setTypeApplying(prev => ({ ...prev, [credentialType]: true }));
    setError(null);

    try {
      // Update all credentials of this type
      await Promise.all(typeCredentials.map(c =>
        api.put('/credentials/' + c.id + '/visibility', { visibility: newVisibility })
      ));

      setCredentials(prev => prev.map(c =>
        c.credential_type === credentialType ? { ...c, visibility: newVisibility } : c
      ));

      setMessage('Access setting applied to all ' + CREDENTIAL_TYPE_LABELS[credentialType]);
      setTimeout(() => setMessage(null), 3000);
    } catch (err) {
      console.error('Failed to apply type settings:', err);
      setError('Failed to apply settings');
    } finally {
      setTypeApplying(prev => ({ ...prev, [credentialType]: false }));
    }
  }

  const credentialsByType = {
    insurance_card: credentials.filter(c => c.credential_type === 'insurance_card'),
    government_id: credentials.filter(c => c.credential_type === 'government_id'),
    advance_directive: credentials.filter(c => c.credential_type === 'advance_directive'),
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Messages */}
        {message && (
          <div className="mb-6 bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg flex justify-between items-center">
            <span>{message}</span>
            <button onClick={() => setMessage(null)} className="text-green-500 hover:text-green-700" aria-label="Dismiss message">&times;</button>
          </div>
        )}
        {error && (
          <div className="mb-6 bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg flex justify-between items-center">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="text-red-500 hover:text-red-700" aria-label="Dismiss error">&times;</button>
          </div>
        )}

        {/* Info Banner */}
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
          <h3 className="text-blue-800 font-semibold mb-1">About Access Settings</h3>
          <p className="text-sm text-blue-700">
            Control who can see each of your credentials. <strong>Private</strong> means only you can access it.{' '}
            <strong>Emergency Visible</strong> allows first responders to view it during emergency card scans.{' '}
            <strong>Always Visible</strong> makes it accessible to any authorized provider with consent.
          </p>
        </div>

        {loading ? (
          <div className="text-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500 mx-auto mb-3"></div>
            <p className="text-gray-500">Loading credential settings...</p>
          </div>
        ) : (
          <div className="space-y-6">
            {Object.keys(credentialsByType).map((credType) => (
              <div key={credType} className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
                {/* Type Header */}
                <div className="flex items-center space-x-3 mb-4">
                  <div className="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center">
                    {CREDENTIAL_TYPE_ICONS[credType]}
                  </div>
                  <div>
                    <h2 className="text-lg font-semibold text-gray-900">{CREDENTIAL_TYPE_LABELS[credType]}</h2>
                    <p className="text-sm text-gray-500">{CREDENTIAL_TYPE_DESCRIPTIONS[credType]}</p>
                  </div>
                </div>

                {/* Bulk Type Settings */}
                <div className="bg-gray-50 rounded-lg p-4 mb-4 border border-gray-200">
                  <div className="flex items-center justify-between">
                    <div className="flex-1 mr-4">
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        Default Access for All {CREDENTIAL_TYPE_LABELS[credType]}
                      </label>
                      <select
                        value={typeSettings[credType]}
                        onChange={(e) => setTypeSettings(prev => ({ ...prev, [credType]: e.target.value }))}
                        className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-sky-500 focus:border-sky-500"
                      >
                        {VISIBILITY_OPTIONS.map(opt => (
                          <option key={opt.value} value={opt.value}>{opt.label} — {opt.description}</option>
                        ))}
                      </select>
                    </div>
                    <button
                      onClick={() => handleApplyToType(credType)}
                      disabled={typeApplying[credType]}
                      className="bg-sky-500 hover:bg-sky-600 disabled:opacity-50 text-white px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap"
                    >
                      {typeApplying[credType] ? 'Applying...' : 'Apply to All'}
                    </button>
                  </div>
                </div>

                {/* Individual Credentials */}
                {credentialsByType[credType].length === 0 ? (
                  <p className="text-gray-400 text-sm italic">No {CREDENTIAL_TYPE_LABELS[credType].toLowerCase()} stored yet.</p>
                ) : (
                  <div className="space-y-3">
                    {credentialsByType[credType].map((cred) => (
                      <div key={cred.id} className="flex items-center justify-between py-3 px-4 border border-gray-200 rounded-lg hover:border-gray-300 transition-colors">
                        <div className="flex items-center space-x-3">
                          <div>
                            <div className="font-medium text-gray-900 text-sm">{cred.title || cred.carrier || 'Credential'}</div>
                            <div className="text-xs text-gray-500">
                              Added {new Date(cred.created_at).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' })}
                            </div>
                          </div>
                          <VisibilityBadge visibility={cred.visibility} />
                        </div>
                        <div className="flex items-center space-x-2">
                          <select
                            value={cred.visibility || 'private'}
                            onChange={(e) => handleVisibilityChange(cred.id, e.target.value)}
                            disabled={saving[cred.id]}
                            className="border border-gray-300 rounded-lg px-3 py-1.5 text-sm focus:ring-sky-500 focus:border-sky-500 disabled:opacity-50"
                          >
                            {VISIBILITY_OPTIONS.map(opt => (
                              <option key={opt.value} value={opt.value}>{opt.label}</option>
                            ))}
                          </select>
                          {saving[cred.id] && (
                            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-sky-500"></div>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
