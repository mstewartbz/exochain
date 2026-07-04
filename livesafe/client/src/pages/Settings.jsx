import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import Navbar from '../components/Navbar';
import api from '../services/api';

const BLOOD_TYPES = ['', 'A+', 'A-', 'B+', 'B-', 'AB+', 'AB-', 'O+', 'O-'];
const DNR_OPTIONS = ['not_specified', 'full_code', 'dnr', 'dnr_comfort_only', 'limited_intervention'];

export default function Settings() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');
  const [alertSensitivity, setAlertSensitivity] = useState('always');
  const [alertSms, setAlertSms] = useState(true);
  const [alertPush, setAlertPush] = useState(true);
  const [alertEmail, setAlertEmail] = useState(true);
  const [alertSaving, setAlertSaving] = useState(false);
  const [alertMessage, setAlertMessage] = useState('');

  // Consent defaults state (Feature #160)
  const [consentDefaultScope, setConsentDefaultScope] = useState('basic_health');
  const [consentDefaultDuration, setConsentDefaultDuration] = useState(30);
  const [consentSaving, setConsentSaving] = useState(false);
  const [consentMessage, setConsentMessage] = useState('');

  // DID copy state (Feature #160)
  const [didCopied, setDidCopied] = useState(false);

  // API timeout test state (Feature #192)
  const [connTestStatus, setConnTestStatus] = useState('idle'); // 'idle'|'testing'|'success'|'timeout'|'error'
  const [connTestMessage, setConnTestMessage] = useState('');
  const lastFailedConfigRef = useRef(null);

  // DB health test state (Feature #194)
  const [dbTestStatus, setDbTestStatus] = useState('idle'); // 'idle'|'testing'|'success'|'unavailable'|'error'
  const [dbTestMessage, setDbTestMessage] = useState('');

  // Delete account state (Feature #227)
  const [deleteAccountPassword, setDeleteAccountPassword] = useState('');
  const [deleteAccountConfirm, setDeleteAccountConfirm] = useState(false);
  const [deleteAccountLoading, setDeleteAccountLoading] = useState(false);
  const [deleteAccountError, setDeleteAccountError] = useState('');
  const [deleteAccountMessage, setDeleteAccountMessage] = useState('');

  // Feature #306: Settings sub-navigation tab
  const [settingsTab, setSettingsTab] = useState('account');

  // Device management state (Feature #179)
  const [devices, setDevices] = useState([]);
  const [devicesLoading, setDevicesLoading] = useState(false);
  const [newDeviceName, setNewDeviceName] = useState('');
  const [deviceRegLoading, setDeviceRegLoading] = useState(false);
  const [deviceMessage, setDeviceMessage] = useState('');
  const [revokingDevice, setRevokingDevice] = useState(null);

  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [dateOfBirth, setDateOfBirth] = useState('');
  const [bloodType, setBloodType] = useState('');
  const [dnrStatus, setDnrStatus] = useState('not_specified');
  const [organDonor, setOrganDonor] = useState(false);

  const [allergies, setAllergies] = useState([]);
  const [medications, setMedications] = useState([]);
  const [conditions, setConditions] = useState([]);
  const [emergencyContacts, setEmergencyContacts] = useState([]);

  const [newAllergy, setNewAllergy] = useState('');
  const [newMedication, setNewMedication] = useState('');
  const [newCondition, setNewCondition] = useState('');

  const fetchDevices = useCallback(async () => {
    setDevicesLoading(true);
    try {
      const res = await api.get('/devices');
      setDevices(res.data.devices || []);
    } catch (err) {
      console.error('Failed to load devices:', err);
    } finally {
      setDevicesLoading(false);
    }
  }, []);

  const handleRegisterDevice = async (e) => {
    e.preventDefault();
    if (!newDeviceName.trim()) return;
    setDeviceRegLoading(true);
    setDeviceMessage('');
    try {
      await api.post('/devices/register', { device_name: newDeviceName.trim() });
      setDeviceMessage('Device registered successfully!');
      setNewDeviceName('');
      fetchDevices();
      setTimeout(() => setDeviceMessage(''), 3000);
    } catch (err) {
      setDeviceMessage('Failed to register device.');
    } finally {
      setDeviceRegLoading(false);
    }
  };

  const handleRevokeDevice = async (deviceId, deviceName) => {
    if (!window.confirm(`Revoke device "${deviceName}"? It will immediately lose access.`)) return;
    setRevokingDevice(deviceId);
    setDeviceMessage('');
    try {
      await api.delete(`/devices/${encodeURIComponent(deviceId)}`, { data: { reason: 'User revoked' } });
      setDeviceMessage(`Device "${deviceName}" revoked successfully.`);
      fetchDevices();
      setTimeout(() => setDeviceMessage(''), 3000);
    } catch (err) {
      setDeviceMessage('Failed to revoke device.');
    } finally {
      setRevokingDevice(null);
    }
  };

  const fetchProfile = useCallback(async () => {
    try {
      setLoading(true);
      const [profileRes, alertRes, consentRes] = await Promise.all([
        api.get('/subscribers/profile'),
        api.get('/subscribers/alert-settings').catch(() => ({ data: { alert_sensitivity: 'always' } })),
        api.get('/subscribers/consent-defaults').catch(() => ({ data: { default_scope: 'basic_health', default_duration_days: 30 } })),
      ]);
      const p = profileRes.data;
      setFirstName(p.first_name || '');
      setLastName(p.last_name || '');
      setDateOfBirth(p.date_of_birth ? p.date_of_birth.substring(0, 10) : '');
      setBloodType(p.blood_type || '');
      setDnrStatus(p.dnr_status || 'not_specified');
      setOrganDonor(p.organ_donor || false);
      setAllergies(p.allergies || []);
      setMedications(p.medications || []);
      setConditions(p.conditions || []);
      setEmergencyContacts(p.emergency_contacts || []);
      setAlertSensitivity(alertRes.data.alert_sensitivity || p.alert_sensitivity || 'always');
      setAlertSms(alertRes.data.sms_alerts !== false);
      setAlertPush(alertRes.data.push_alerts !== false);
      setAlertEmail(alertRes.data.email_alerts !== false);
      setConsentDefaultScope(consentRes.data.default_scope || 'basic_health');
      setConsentDefaultDuration(consentRes.data.default_duration_days || 30);
    } catch (err) {
      console.error('Failed to fetch profile:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchProfile(); fetchDevices(); }, [fetchProfile, fetchDevices]);

  const handleSave = async (e) => {
    e.preventDefault();
    setSaving(true);
    setMessage('');
    try {
      await api.put('/subscribers/profile', {
        first_name: firstName,
        last_name: lastName,
        date_of_birth: dateOfBirth || null,
        blood_type: bloodType || null,
        dnr_status: dnrStatus,
        organ_donor: organDonor,
      });
      setMessage('Settings saved successfully!');
      setTimeout(() => setMessage(''), 3000);
    } catch (err) {
      setMessage('Failed to save settings.');
    } finally {
      setSaving(false);
    }
  };

  const handleSaveAlertSettings = async (e) => {
    e.preventDefault();
    setAlertSaving(true);
    setAlertMessage('');
    try {
      await api.put('/subscribers/alert-settings', {
        alert_sensitivity: alertSensitivity,
        sms_alerts: alertSms,
        push_alerts: alertPush,
        email_alerts: alertEmail,
      });
      setAlertMessage('Alert settings saved!');
      setTimeout(() => setAlertMessage(''), 3000);
    } catch (err) {
      setAlertMessage('Failed to save alert settings.');
    } finally {
      setAlertSaving(false);
    }
  };

  const handleSaveConsentDefaults = async (e) => {
    e.preventDefault();
    setConsentSaving(true);
    setConsentMessage('');
    try {
      await api.put('/subscribers/consent-defaults', {
        default_scope: consentDefaultScope,
        default_duration_days: Number(consentDefaultDuration),
      });
      setConsentMessage('Consent defaults saved!');
      setTimeout(() => setConsentMessage(''), 3000);
    } catch (err) {
      setConsentMessage('Failed to save consent defaults.');
    } finally {
      setConsentSaving(false);
    }
  };

  const handleCopyDid = () => {
    if (user?.did) {
      navigator.clipboard.writeText(user.did).catch(() => {});
      setDidCopied(true);
      setTimeout(() => setDidCopied(false), 2000);
    }
  };

  // Feature #192: Test API connection (fast ping)
  const handleTestConnection = async () => {
    setConnTestStatus('testing');
    setConnTestMessage('');
    try {
      await api.get('/test/ping');
      setConnTestStatus('success');
      setConnTestMessage('Connection successful! API is responding normally.');
      setTimeout(() => setConnTestStatus('idle'), 4000);
    } catch (err) {
      if (err.isTimeout) {
        setConnTestStatus('timeout');
        setConnTestMessage('Request timed out. The API did not respond within the threshold.');
        lastFailedConfigRef.current = err.originalConfig;
      } else {
        setConnTestStatus('error');
        setConnTestMessage(err.message || 'Connection failed.');
      }
    }
  };

  // Feature #192: Test slow API response to demonstrate timeout handling
  const handleTestTimeout = async () => {
    setConnTestStatus('testing');
    setConnTestMessage('Sending request to slow endpoint (5s delay, 3s timeout)...');
    lastFailedConfigRef.current = null;
    try {
      // Use a 3-second timeout override for this specific request — will time out
      await api.get('/test/slow?delay=5000', { timeout: 3000 });
      setConnTestStatus('success');
      setConnTestMessage('Request completed (no timeout occurred).');
    } catch (err) {
      if (err.isTimeout) {
        setConnTestStatus('timeout');
        setConnTestMessage('Request timed out after 3 seconds. This demonstrates graceful timeout handling.');
        lastFailedConfigRef.current = { url: '/test/ping', method: 'get' };
      } else {
        setConnTestStatus('error');
        setConnTestMessage(err.message || 'Request failed.');
      }
    }
  };

  // Feature #194: Test DB unavailability handling
  const handleTestDbUnavailable = async () => {
    setDbTestStatus('testing');
    setDbTestMessage('');
    try {
      await api.get('/test/db-error');
      setDbTestStatus('success');
      setDbTestMessage('No error (unexpected).');
    } catch (err) {
      if (err.isServiceUnavailable || err.status === 503 || err.response?.status === 503) {
        const msg = err.message || err.response?.data?.error || 'Service temporarily unavailable.';
        setDbTestStatus('unavailable');
        setDbTestMessage(msg);
      } else {
        setDbTestStatus('error');
        setDbTestMessage(err.message || 'Unexpected error.');
      }
    }
  };

  // Feature #227: Delete account handler
  const handleDeleteAccount = async (e) => {
    e.preventDefault();
    if (!deleteAccountPassword) {
      setDeleteAccountError('Please enter your password to confirm account deletion.');
      return;
    }
    setDeleteAccountLoading(true);
    setDeleteAccountError('');
    setDeleteAccountMessage('');
    try {
      const res = await api.delete('/subscribers/account', { data: { password: deleteAccountPassword } });
      setDeleteAccountMessage('Account deleted successfully. You will be logged out.');
      // Log out after short delay
      setTimeout(() => {
        logout();
        navigate('/');
      }, 2000);
    } catch (err) {
      setDeleteAccountError(err.response?.data?.error || 'Failed to delete account. Please try again.');
    } finally {
      setDeleteAccountLoading(false);
    }
  };

  const handleRecoverDb = async () => {
    setDbTestStatus('testing');
    setDbTestMessage('Checking database connection...');
    try {
      const r = await api.get('/health');
      if (r.data?.database === 'connected') {
        setDbTestStatus('success');
        setDbTestMessage('✓ Database is online and responding normally.');
        setTimeout(() => setDbTestStatus('idle'), 4000);
      } else {
        setDbTestStatus('unavailable');
        setDbTestMessage('Database is still unavailable. Please try again shortly.');
      }
    } catch (err) {
      setDbTestStatus('unavailable');
      setDbTestMessage('Database is still unavailable: ' + (err.message || ''));
    }
  };

  // Feature #192: Retry after timeout
  const handleRetryConnection = async () => {
    setConnTestStatus('testing');
    setConnTestMessage('Retrying connection...');
    try {
      await api.get('/test/ping');
      setConnTestStatus('success');
      setConnTestMessage('✓ Connection restored! API is responding normally.');
      lastFailedConfigRef.current = null;
      setTimeout(() => setConnTestStatus('idle'), 4000);
    } catch (err) {
      if (err.isTimeout) {
        setConnTestStatus('timeout');
        setConnTestMessage('Still timing out. Please check your connection.');
      } else {
        setConnTestStatus('error');
        setConnTestMessage(err.message || 'Retry failed.');
      }
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="flex justify-center py-20">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main id="main-content" tabIndex={-1} className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mb-6">
          <h1 className="text-2xl font-bold text-gray-900">Settings</h1>
          <p className="text-gray-600 mt-1">Manage your profile and account settings</p>
        </div>

        {/* Feature #306: Settings sub-navigation */}
        <div className="mb-6 border-b border-gray-200" data-testid="settings-subnav">
          <nav className="-mb-px flex gap-1" aria-label="Settings sections">
            {[
              { key: 'account', label: 'Account Settings' },
              { key: 'did', label: 'DID Management' },
              { key: 'alerts', label: 'Alert Preferences' },
            ].map(tab => (
              <button
                key={tab.key}
                onClick={() => setSettingsTab(tab.key)}
                data-testid={`settings-tab-${tab.key}`}
                aria-selected={settingsTab === tab.key}
                className={`px-4 py-2 text-sm font-medium rounded-t-lg border-b-2 transition-colors focus:outline-none focus:ring-2 focus:ring-sky-500 focus:ring-offset-1 ${
                  settingsTab === tab.key
                    ? 'border-sky-500 text-sky-600 bg-sky-50'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                {tab.label}
              </button>
            ))}
          </nav>
        </div>

        {/* === ACCOUNT SETTINGS TAB === */}
        {settingsTab === 'account' && (
          <div data-testid="settings-content-account">

        {/* Account Settings */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="account-settings-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Account Settings</h2>
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-500 w-24">Email:</span>
              <span className="text-sm text-gray-900">{user?.email}</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-500 w-24">Role:</span>
              <span className="text-sm text-gray-900 capitalize">{user?.role || 'subscriber'}</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-500 w-24">Account Tier:</span>
              {user?.is_hero || user?.is_military ? (
                <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-100 text-emerald-800 border border-emerald-200" data-testid="heroes-free-tier-badge">
                  Heroes — Free Forever
                </span>
              ) : (
                <span className="text-sm text-gray-900" data-testid="account-tier-free">
                  Free
                </span>
              )}
            </div>
          </div>
          {(user?.is_hero || user?.is_military) && (
            <div className="mt-4 p-3 bg-emerald-50 border border-emerald-200 rounded-lg" data-testid="heroes-free-tier-info">
              <p className="text-xs text-emerald-700 font-medium">
                Your Heroes status entitles you to a <strong>free forever</strong> LiveSafe account.
                No payment required, no expiration.
              </p>
            </div>
          )}
          <div className="mt-4 flex gap-3">
            <Link to="/credentials/settings" className="text-sm text-sky-700 hover:underline">
              Credential Vault Settings →
            </Link>
          </div>
        </div>

          </div>
        )}

        {/* === DID MANAGEMENT TAB === */}
        {settingsTab === 'did' && (
          <div data-testid="settings-content-did">

        {/* DID Management */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="did-management-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-1">DID Management</h2>
          <p className="text-sm text-gray-500 mb-4">Your decentralized identifier (DID) identifies your LiveSafe account for consent and recovery workflows.</p>
          <div className="bg-sky-50 rounded-lg p-4 border border-sky-100">
            <div className="flex items-start gap-3">
              <div className="flex-1 min-w-0">
                <div className="text-xs font-medium text-sky-700 mb-1">Your Decentralized Identifier (DID)</div>
                <code className="text-xs text-sky-900 break-all">{user?.did}</code>
              </div>
              <button
                onClick={handleCopyDid}
                className="flex-shrink-0 px-3 py-1.5 bg-sky-100 hover:bg-sky-200 text-sky-700 text-xs rounded-lg transition font-medium"
                data-testid="copy-did-btn"
              >
                {didCopied ? '✓ Copied' : 'Copy DID'}
              </button>
            </div>
          </div>
          <div className="mt-3 space-y-1">
            <div className="flex items-center gap-2 text-sm text-gray-600">
              <span className="text-sky-500">i</span>
              <span>EXOCHAIN anchoring remains inactive until a verified adapter path is invoked</span>
            </div>
            <div className="flex items-center gap-2 text-sm text-gray-600">
              <span className="text-emerald-500">✓</span>
              <span>Protected by your PACE trustee network</span>
            </div>
            <div className="flex items-center gap-2 text-sm text-gray-600">
              <span className="text-sky-500">🔑</span>
              <span>Key shards distributed across your trustees</span>
            </div>
          </div>
        </div>
          </div>
        )}

        {/* === ACCOUNT SETTINGS TAB (continued: profile, devices, etc.) === */}
        {settingsTab === 'account' && (
          <div data-testid="settings-content-account-extended">

        {/* Device Management (Feature #179) */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="device-management-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-1">Device Management</h2>
          <p className="text-sm text-gray-500 mb-4">Manage devices that can access your account. Revoke access from lost or compromised devices.</p>

          {deviceMessage && (
            <div className={`mb-4 p-3 rounded-lg text-sm ${deviceMessage.includes('success') || deviceMessage.includes('revoked') ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'}`}
              data-testid="device-message" role="status" aria-live="polite">
              {deviceMessage}
            </div>
          )}

          {/* Register new device */}
          <form onSubmit={handleRegisterDevice} className="flex gap-2 mb-4">
            <input
              type="text"
              value={newDeviceName}
              onChange={e => setNewDeviceName(e.target.value)}
              placeholder="Device name (e.g., My iPhone)"
              className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
              data-testid="device-name-input"
            />
            <button
              type="submit"
              disabled={deviceRegLoading || !newDeviceName.trim()}
              className="px-4 py-2 bg-sky-600 text-white text-sm rounded-lg hover:bg-sky-700 transition disabled:opacity-50"
              data-testid="register-device-btn"
            >
              {deviceRegLoading ? 'Registering…' : 'Register Device'}
            </button>
          </form>

          {/* Device list */}
          {devicesLoading ? (
            <div className="text-sm text-gray-500">Loading devices…</div>
          ) : devices.length === 0 ? (
            <div className="text-sm text-gray-500 italic" data-testid="no-devices-msg">No devices registered.</div>
          ) : (
            <div className="space-y-2" data-testid="devices-list">
              {devices.map(device => (
                <div
                  key={device.device_id}
                  className={`flex items-center justify-between p-3 rounded-lg border ${device.is_active ? 'border-emerald-200 bg-emerald-50' : 'border-gray-200 bg-gray-50'}`}
                  data-testid={`device-item-${device.device_id}`}
                >
                  <div>
                    <div className="text-sm font-medium text-gray-900">{device.device_name}</div>
                    <div className="text-xs text-gray-500">
                      {device.is_active ? (
                        <span className="text-emerald-600 font-medium">● Active</span>
                      ) : (
                        <span className="text-red-500 font-medium">✕ Revoked {device.revoked_at ? new Date(device.revoked_at).toLocaleDateString() : ''}</span>
                      )}
                      {device.last_used_at && (
                        <span className="ml-2">· Last used: {new Date(device.last_used_at).toLocaleDateString()}</span>
                      )}
                    </div>
                  </div>
                  {device.is_active && (
                    <button
                      onClick={() => handleRevokeDevice(device.device_id, device.device_name)}
                      disabled={revokingDevice === device.device_id}
                      className="px-3 py-1 bg-red-100 hover:bg-red-200 text-red-700 text-xs rounded-lg transition disabled:opacity-50"
                      data-testid={`revoke-device-btn-${device.device_id}`}
                    >
                      {revokingDevice === device.device_id ? 'Revoking…' : 'Revoke'}
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Profile Form */}
        <form onSubmit={handleSave} className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Profile Information</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">First Name</label>
              <input
                type="text"
                value={firstName}
                onChange={e => setFirstName(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Last Name</label>
              <input
                type="text"
                value={lastName}
                onChange={e => setLastName(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Date of Birth</label>
              <input
                type="date"
                value={dateOfBirth}
                onChange={e => setDateOfBirth(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Blood Type</label>
              <select
                value={bloodType}
                onChange={e => setBloodType(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              >
                {BLOOD_TYPES.map(bt => (
                  <option key={bt} value={bt}>{bt || 'Not specified'}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">DNR Status</label>
              <select
                value={dnrStatus}
                onChange={e => setDnrStatus(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              >
                {DNR_OPTIONS.map(opt => (
                  <option key={opt} value={opt}>{opt.replace(/_/g, ' ')}</option>
                ))}
              </select>
            </div>
            <div className="flex items-center gap-2 pt-6">
              <input
                type="checkbox"
                id="organDonor"
                checked={organDonor}
                onChange={e => setOrganDonor(e.target.checked)}
                className="w-4 h-4 text-sky-600 border-gray-300 rounded"
              />
              <label htmlFor="organDonor" className="text-sm font-medium text-gray-700">Organ Donor</label>
            </div>
          </div>

          {message && (
            <div className={`mb-4 p-3 rounded-lg text-sm ${message.includes('success') ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'}`} role="status" aria-live="polite">
              {message}
            </div>
          )}

          <button
            type="submit"
            disabled={saving}
            className="px-6 py-2 bg-sky-600 text-white rounded-lg hover:bg-sky-700 transition disabled:opacity-50"
          >
            {saving ? 'Saving...' : 'Save Settings'}
          </button>
        </form>

        {/* Consent Defaults */}
        <form onSubmit={handleSaveConsentDefaults} className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="consent-defaults-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-1">Consent Defaults</h2>
          <p className="text-sm text-gray-500 mb-4">Set default values used when granting provider access to your health data.</p>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Default Access Scope</label>
              <select
                value={consentDefaultScope}
                onChange={e => setConsentDefaultScope(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="consent-scope-select"
              >
                <option value="basic_health">Basic Health — vitals, allergies, medications</option>
                <option value="full_health">Full Health — complete medical history</option>
                <option value="emergency_only">Emergency Only — emergency data only</option>
                <option value="research">Research — de-identified research data</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Default Duration</label>
              <select
                value={consentDefaultDuration}
                onChange={e => setConsentDefaultDuration(Number(e.target.value))}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="consent-duration-select"
              >
                <option value={7}>7 days</option>
                <option value={30}>30 days</option>
                <option value={90}>90 days</option>
                <option value={180}>180 days</option>
                <option value={365}>365 days (1 year)</option>
              </select>
            </div>
          </div>

          {consentMessage && (
            <div className={`mb-4 p-3 rounded-lg text-sm ${consentMessage.includes('saved') ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'}`}
              data-testid="consent-defaults-message" role="status" aria-live="polite">
              {consentMessage}
            </div>
          )}

          <button
            type="submit"
            disabled={consentSaving}
            className="px-6 py-2 bg-sky-600 text-white rounded-lg hover:bg-sky-700 transition disabled:opacity-50"
            data-testid="save-consent-defaults-btn"
          >
            {consentSaving ? 'Saving...' : 'Save Consent Defaults'}
          </button>
        </form>

          </div>
        )}

        {/* === ALERT PREFERENCES TAB === */}
        {settingsTab === 'alerts' && (
          <div data-testid="settings-content-alerts">

        {/* Alert Settings */}
        <form onSubmit={handleSaveAlertSettings} className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="alert-settings-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-1">Alert Settings</h2>
          <p className="text-sm text-gray-500 mb-4">Control when and how you receive notifications about your emergency card being scanned.</p>

          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 mb-2">Alert Sensitivity</label>
            <select
              value={alertSensitivity}
              onChange={e => setAlertSensitivity(e.target.value)}
              className="w-full sm:w-auto px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
              data-testid="alert-sensitivity-select"
            >
              <option value="always">Always — notify me on every scan</option>
              <option value="emergency-only">Emergency Only — notify me on emergency scans</option>
              <option value="off">Off — no scan notifications</option>
            </select>
            <p className="text-xs text-gray-500 mt-1">
              {alertSensitivity === 'always' && 'You will receive a notification every time your card is scanned.'}
              {alertSensitivity === 'emergency-only' && 'You will only receive notifications for emergency card scans.'}
              {alertSensitivity === 'off' && 'You will not receive any scan notifications. PACE trustee alerts are not affected.'}
            </p>
          </div>

          {/* Notification Channels — Feature #280: SMS, push, email enabled by default */}
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 mb-2">Notification Channels</label>
            <p className="text-xs text-gray-500 mb-3">Choose how you want to receive alerts. All channels are enabled by default.</p>
            <div className="space-y-3" data-testid="alert-channels">
              <label className="flex items-center gap-3 cursor-pointer" data-testid="alert-channel-sms-label">
                <input
                  type="checkbox"
                  checked={alertSms}
                  onChange={e => setAlertSms(e.target.checked)}
                  className="h-4 w-4 text-sky-600 border-gray-300 rounded focus:ring-sky-500"
                  data-testid="alert-sms-checkbox"
                />
                <span className="text-sm text-gray-700">
                  <span className="font-medium">SMS</span> — text messages to your verified phone number
                </span>
              </label>
              <label className="flex items-center gap-3 cursor-pointer" data-testid="alert-channel-push-label">
                <input
                  type="checkbox"
                  checked={alertPush}
                  onChange={e => setAlertPush(e.target.checked)}
                  className="h-4 w-4 text-sky-600 border-gray-300 rounded focus:ring-sky-500"
                  data-testid="alert-push-checkbox"
                />
                <span className="text-sm text-gray-700">
                  <span className="font-medium">Push notifications</span> — in-app and browser notifications
                </span>
              </label>
              <label className="flex items-center gap-3 cursor-pointer" data-testid="alert-channel-email-label">
                <input
                  type="checkbox"
                  checked={alertEmail}
                  onChange={e => setAlertEmail(e.target.checked)}
                  className="h-4 w-4 text-sky-600 border-gray-300 rounded focus:ring-sky-500"
                  data-testid="alert-email-checkbox"
                />
                <span className="text-sm text-gray-700">
                  <span className="font-medium">Email</span> — notifications sent to your registered email address
                </span>
              </label>
            </div>
            {!alertSms && !alertPush && !alertEmail && (
              <p className="mt-2 text-xs text-amber-600 bg-amber-50 rounded-lg px-3 py-2" data-testid="no-channels-warning">
                ⚠️ No notification channels are enabled. You will not receive any alerts.
              </p>
            )}
          </div>

          {alertMessage && (
            <div className={`mb-4 p-3 rounded-lg text-sm ${alertMessage.includes('saved') ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'}`}
              data-testid="alert-settings-message" role="status" aria-live="polite">
              {alertMessage}
            </div>
          )}

          <button
            type="submit"
            disabled={alertSaving}
            className="px-6 py-2 bg-sky-600 text-white rounded-lg hover:bg-sky-700 transition disabled:opacity-50"
            data-testid="save-alert-settings-btn"
          >
            {alertSaving ? 'Saving...' : 'Save Alert Settings'}
          </button>
        </form>
          </div>
        )}

        {/* API Health, DB Health, Delete Account — Account tab only */}
        {settingsTab === 'account' && (
          <>
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="api-health-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-1">API Connection Health</h2>
          <p className="text-sm text-gray-500 mb-4">
            Test your connection to the LiveSafe API. Long-running requests will automatically time out
            after <strong>30 seconds</strong> with a user-friendly message and retry option.
          </p>

          {/* Status display */}
          {connTestStatus === 'testing' && (
            <div className="mb-4 p-3 bg-sky-50 border border-sky-200 rounded-lg flex items-center gap-2" data-testid="connection-testing">
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-sky-500 flex-shrink-0"></div>
              <span className="text-sm text-sky-700">{connTestMessage || 'Testing connection...'}</span>
            </div>
          )}

          {connTestStatus === 'success' && (
            <div className="mb-4 p-3 bg-emerald-50 border border-emerald-200 rounded-lg flex items-center gap-2" data-testid="connection-success">
              <span className="text-emerald-500">✓</span>
              <span className="text-sm text-emerald-700">{connTestMessage}</span>
            </div>
          )}

          {connTestStatus === 'timeout' && (
            <div className="mb-4 p-4 bg-amber-50 border border-amber-200 rounded-lg" data-testid="timeout-message">
              <div className="flex items-start gap-3">
                <span className="text-amber-500 text-lg flex-shrink-0">⏱</span>
                <div className="flex-1">
                  <div className="text-sm font-semibold text-amber-800 mb-1">Request Timed Out</div>
                  <div className="text-sm text-amber-700 mb-3">{connTestMessage}</div>
                  <div className="flex gap-2">
                    <button
                      onClick={handleRetryConnection}
                      className="px-4 py-1.5 bg-amber-600 hover:bg-amber-700 text-white text-sm rounded-lg transition font-medium"
                      data-testid="retry-btn"
                    >
                      Retry Connection
                    </button>
                    <button
                      onClick={() => { setConnTestStatus('idle'); setConnTestMessage(''); }}
                      className="px-4 py-1.5 bg-white border border-amber-300 hover:bg-amber-50 text-amber-700 text-sm rounded-lg transition"
                      data-testid="dismiss-timeout-btn"
                    >
                      Dismiss
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {connTestStatus === 'error' && (
            <div className="mb-4 p-4 bg-red-50 border border-red-200 rounded-lg" data-testid="connection-error" role="alert">
              <div className="flex items-start gap-3">
                <span className="text-red-500 flex-shrink-0">✕</span>
                <div className="flex-1">
                  <div className="text-sm font-semibold text-red-800 mb-1">Connection Error</div>
                  <div className="text-sm text-red-700 mb-3">{connTestMessage}</div>
                  <button
                    onClick={handleRetryConnection}
                    className="px-4 py-1.5 bg-red-600 hover:bg-red-700 text-white text-sm rounded-lg transition font-medium"
                    data-testid="retry-btn"
                  >
                    Retry
                  </button>
                </div>
              </div>
            </div>
          )}

          <div className="flex flex-wrap gap-3">
            <button
              onClick={handleTestConnection}
              disabled={connTestStatus === 'testing'}
              className="px-4 py-2 bg-sky-600 hover:bg-sky-700 disabled:opacity-50 text-white text-sm rounded-lg transition font-medium"
              data-testid="test-connection-btn"
            >
              {connTestStatus === 'testing' ? 'Testing...' : 'Test Connection'}
            </button>
            <button
              onClick={handleTestTimeout}
              disabled={connTestStatus === 'testing'}
              className="px-4 py-2 bg-amber-500 hover:bg-amber-600 disabled:opacity-50 text-white text-sm rounded-lg transition font-medium"
              data-testid="test-timeout-btn"
            >
              Simulate Timeout
            </button>
          </div>
          <p className="text-xs text-gray-400 mt-2">
            "Simulate Timeout" sends a request that exceeds the 3-second threshold to demonstrate timeout handling.
          </p>
        </div>

        {/* Database Availability Testing — Feature #194 */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6 mb-6" data-testid="db-health-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-1">Database Health</h2>
          <p className="text-sm text-gray-500 mb-4">
            Verify the application handles database downtime gracefully with user-friendly error messages.
            When the database is temporarily unavailable, the app displays a clear error and recovers automatically.
          </p>

          {/* DB Status display */}
          {dbTestStatus === 'testing' && (
            <div className="mb-4 p-3 bg-sky-50 border border-sky-200 rounded-lg flex items-center gap-2" data-testid="db-testing">
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-sky-500 flex-shrink-0"></div>
              <span className="text-sm text-sky-700">{dbTestMessage || 'Checking database...'}</span>
            </div>
          )}

          {dbTestStatus === 'success' && (
            <div className="mb-4 p-3 bg-emerald-50 border border-emerald-200 rounded-lg flex items-center gap-2" data-testid="db-connected">
              <span className="text-emerald-500">✓</span>
              <span className="text-sm text-emerald-700">{dbTestMessage}</span>
            </div>
          )}

          {dbTestStatus === 'unavailable' && (
            <div className="mb-4 p-4 bg-red-50 border border-red-200 rounded-lg" data-testid="db-unavailable-message">
              <div className="flex items-start gap-3">
                <span className="text-red-500 text-lg flex-shrink-0">⚠</span>
                <div className="flex-1">
                  <div className="text-sm font-semibold text-red-800 mb-1">Database Unavailable</div>
                  <div className="text-sm text-red-700 mb-3">{dbTestMessage}</div>
                  <div className="flex gap-2">
                    <button
                      onClick={handleRecoverDb}
                      className="px-4 py-1.5 bg-red-600 hover:bg-red-700 text-white text-sm rounded-lg transition font-medium"
                      data-testid="db-recover-btn"
                    >
                      Check Recovery
                    </button>
                    <button
                      onClick={() => { setDbTestStatus('idle'); setDbTestMessage(''); }}
                      className="px-4 py-1.5 bg-white border border-red-300 hover:bg-red-50 text-red-700 text-sm rounded-lg transition"
                    >
                      Dismiss
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {dbTestStatus === 'error' && (
            <div className="mb-4 p-3 bg-orange-50 border border-orange-200 rounded-lg" data-testid="db-error">
              <span className="text-sm text-orange-700">{dbTestMessage}</span>
            </div>
          )}

          <button
            onClick={handleTestDbUnavailable}
            disabled={dbTestStatus === 'testing'}
            className="px-4 py-2 bg-red-500 hover:bg-red-600 disabled:opacity-50 text-white text-sm rounded-lg transition font-medium"
            data-testid="test-db-unavailable-btn"
          >
            Simulate DB Unavailable
          </button>
          <p className="text-xs text-gray-400 mt-2">
            Demonstrates how the app handles a database connection failure — shows user-friendly error, not raw server errors.
          </p>
        </div>

        {/* Danger Zone: Delete Account — Feature #227 */}
        <div className="bg-white rounded-xl shadow-sm border border-red-200 p-6 mb-6" data-testid="delete-account-section">
          <h2 className="text-lg font-semibold text-red-700 mb-1">⚠ Danger Zone: Delete Account</h2>
          <p className="text-sm text-gray-600 mb-4">
            Permanently delete your LiveSafe account. This will remove all associated health records, credentials, PACE
            trustee relationships, and consent grants. A local audit receipt will remain append-only while EXOCHAIN
            anchoring stays inactive until a verified adapter path is invoked. <strong>This
            action cannot be undone.</strong>
          </p>

          {!deleteAccountConfirm ? (
            <button
              onClick={() => setDeleteAccountConfirm(true)}
              className="px-4 py-2 bg-red-100 hover:bg-red-200 text-red-700 border border-red-300 text-sm rounded-lg transition font-medium"
              data-testid="show-delete-account-btn"
            >
              Delete My Account
            </button>
          ) : (
            <form onSubmit={handleDeleteAccount} className="space-y-4" data-testid="delete-account-form">
              <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
                <p className="text-sm text-red-800 font-semibold mb-1">⚠ This will permanently delete:</p>
                <ul className="text-xs text-red-700 space-y-0.5 list-disc list-inside ml-2">
                  <li>All health records and uploaded files</li>
                  <li>All credentials and insurance cards</li>
                  <li>PACE trustee relationships and key shards</li>
                  <li>All provider consent grants</li>
                  <li>Your QR/NFC emergency card</li>
                  <li>Your subscriber profile and DID</li>
                </ul>
                <p className="text-xs text-red-600 mt-2 font-medium">
                  A local audit receipt will remain append-only for compliance.
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Enter your password to confirm deletion
                </label>
                <input
                  type="password"
                  value={deleteAccountPassword}
                  onChange={e => setDeleteAccountPassword(e.target.value)}
                  placeholder="Your current password"
                  className="w-full border border-red-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-red-400 focus:border-red-400 text-sm"
                  data-testid="delete-account-password"
                />
              </div>

              {deleteAccountError && (
                <div className="p-3 bg-red-50 border border-red-200 text-red-700 text-sm rounded-lg" data-testid="delete-account-error" role="alert">
                  {deleteAccountError}
                </div>
              )}
              {deleteAccountMessage && (
                <div className="p-3 bg-green-50 border border-green-200 text-green-700 text-sm rounded-lg" data-testid="delete-account-success">
                  {deleteAccountMessage}
                </div>
              )}

              <div className="flex gap-3">
                <button
                  type="submit"
                  disabled={deleteAccountLoading}
                  className="px-4 py-2 bg-red-600 hover:bg-red-700 disabled:opacity-50 text-white text-sm rounded-lg transition font-medium"
                  data-testid="confirm-delete-account-btn"
                >
                  {deleteAccountLoading ? 'Deleting...' : 'Permanently Delete My Account'}
                </button>
                <button
                  type="button"
                  onClick={() => { setDeleteAccountConfirm(false); setDeleteAccountPassword(''); setDeleteAccountError(''); }}
                  className="px-4 py-2 bg-white border border-gray-300 hover:bg-gray-50 text-gray-700 text-sm rounded-lg transition"
                  data-testid="cancel-delete-account-btn"
                >
                  Cancel
                </button>
              </div>
            </form>
          )}
        </div>
          </>
        )}

      </main>
    </div>
  );
}
