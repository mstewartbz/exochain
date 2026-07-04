import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';

const AGENCY_TYPES = [
  { value: 'fire', label: 'Fire Department' },
  { value: 'ems', label: 'EMS / Ambulance' },
  { value: 'hospital', label: 'Hospital / ER' },
  { value: 'police', label: 'Police / Law Enforcement' },
  { value: 'military', label: 'Military' },
];

function AgencyRegister() {
  const [agencyName, setAgencyName] = useState('');
  const [agencyType, setAgencyType] = useState('');
  const [credentials, setCredentials] = useState('');
  const [adminEmail, setAdminEmail] = useState('');
  const [adminPassword, setAdminPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const validatePassword = (pwd) => {
    if (pwd.length < 8) return 'Password must be at least 8 characters';
    if (!/[A-Z]/.test(pwd)) return 'Password must contain at least one uppercase letter';
    if (!/[a-z]/.test(pwd)) return 'Password must contain at least one lowercase letter';
    if (!/[0-9]/.test(pwd)) return 'Password must contain at least one number';
    return null;
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    setError('');

    if (!agencyName.trim()) {
      setError('Agency name is required');
      return;
    }

    if (!agencyType) {
      setError('Agency type is required');
      return;
    }

    if (!adminEmail || !adminPassword) {
      setError('Admin email and password are required');
      return;
    }

    var passwordError = validatePassword(adminPassword);
    if (passwordError) {
      setError(passwordError);
      return;
    }

    if (adminPassword !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }

    setLoading(true);
    try {
      var response = await api.post('/auth/agency/register', {
        name: agencyName.trim(),
        type: agencyType,
        admin_email: adminEmail,
        admin_password: adminPassword,
        credentials: credentials.trim() || undefined,
      });

      var data = response.data;
      // Store the admin token so they're logged in
      localStorage.setItem('livesafe_responder_token', data.token);
      localStorage.setItem('livesafe_responder_user', JSON.stringify(data.user));
      // Navigate to dashboard - will need to refresh auth context
      window.location.href = '/dashboard';
    } catch (err) {
      if (err.response?.data?.error) {
        setError(err.response.data.error);
      } else {
        setError('Agency registration failed. Please try again.');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-red-50 to-white flex items-center justify-center px-4 py-12">
      <div className="w-full max-w-lg">
        {/* Logo/Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-red-600">
            LiveSafe<span className="text-amber-500">.ai</span>
          </h1>
          <p className="mt-2 text-gray-600 text-lg font-medium">
            Agency Registration
          </p>
          <p className="mt-1 text-sm text-green-600 font-semibold">
            Register your fire, EMS, or hospital agency
          </p>
        </div>

        {/* Registration Form */}
        <div className="bg-white rounded-2xl shadow-lg p-8">
          <h2 className="text-2xl font-semibold text-gray-900 mb-6">Register New Agency</h2>

          {error && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm" role="alert">
              {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Agency Details */}
            <div className="border-b border-gray-200 pb-4 mb-4">
              <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Agency Details</h3>
              <div className="space-y-3">
                <div>
                  <label htmlFor="agencyName" className="block text-sm font-medium text-gray-700 mb-1">
                    Agency Name <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="agencyName"
                    name="agencyName"
                    type="text"
                    required
                    value={agencyName}
                    onChange={(e) => setAgencyName(e.target.value)}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                    placeholder="e.g. City Fire Department, Metro EMS"
                  />
                </div>

                <div>
                  <label htmlFor="agencyType" className="block text-sm font-medium text-gray-700 mb-1">
                    Agency Type <span className="text-red-500">*</span>
                  </label>
                  <select
                    id="agencyType"
                    name="agencyType"
                    required
                    value={agencyType}
                    onChange={(e) => setAgencyType(e.target.value)}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base bg-white"
                  >
                    <option value="">Select agency type...</option>
                    {AGENCY_TYPES.map((t) => (
                      <option key={t.value} value={t.value}>{t.label}</option>
                    ))}
                  </select>
                </div>

                <div>
                  <label htmlFor="credentials" className="block text-sm font-medium text-gray-700 mb-1">
                    Agency Credentials / License
                  </label>
                  <input
                    id="credentials"
                    name="credentials"
                    type="text"
                    value={credentials}
                    onChange={(e) => setCredentials(e.target.value)}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                    placeholder="e.g. State license #, FDID"
                  />
                </div>
              </div>
            </div>

            {/* Admin Account */}
            <div>
              <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Administrator Account</h3>
              <div className="space-y-3">
                <div>
                  <label htmlFor="adminEmail" className="block text-sm font-medium text-gray-700 mb-1">
                    Admin Email <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="adminEmail"
                    name="adminEmail"
                    type="email"
                    autoComplete="email"
                    required
                    value={adminEmail}
                    onChange={(e) => setAdminEmail(e.target.value)}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                    placeholder="admin@agency.gov"
                  />
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label htmlFor="adminPassword" className="block text-sm font-medium text-gray-700 mb-1">
                      Password <span className="text-red-500">*</span>
                    </label>
                    <input
                      id="adminPassword"
                      name="adminPassword"
                      type="password"
                      autoComplete="new-password"
                      required
                      value={adminPassword}
                      onChange={(e) => setAdminPassword(e.target.value)}
                      className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                      placeholder="Min 8 chars"
                    />
                  </div>
                  <div>
                    <label htmlFor="confirmPassword" className="block text-sm font-medium text-gray-700 mb-1">
                      Confirm <span className="text-red-500">*</span>
                    </label>
                    <input
                      id="confirmPassword"
                      name="confirmPassword"
                      type="password"
                      autoComplete="new-password"
                      required
                      value={confirmPassword}
                      onChange={(e) => setConfirmPassword(e.target.value)}
                      className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                      placeholder="Repeat password"
                    />
                  </div>
                </div>
                <p className="text-xs text-gray-500">
                  Must contain uppercase, lowercase, and a number. You will be the agency administrator.
                </p>
              </div>
            </div>

            <button
              type="submit"
              disabled={loading}
              className="w-full py-3 px-4 bg-red-500 hover:bg-red-600 disabled:bg-red-300 text-white font-semibold rounded-lg transition duration-200 text-base focus:ring-2 focus:ring-red-500 focus:ring-offset-2 mt-6"
            >
              {loading ? 'Registering Agency...' : 'Register Agency'}
            </button>
          </form>

          <div className="mt-6 text-center space-y-2">
            <p className="text-sm text-gray-600">
              Already have a responder account?{' '}
              <Link to="/login" className="text-red-600 hover:text-red-700 font-medium">
                Sign in
              </Link>
            </p>
            <p className="text-sm text-gray-600">
              Individual first responder?{' '}
              <Link to="/register" className="text-red-600 hover:text-red-700 font-medium">
                Register here
              </Link>
            </p>
          </div>
        </div>

        {/* Footer */}
        <p className="mt-6 text-center text-xs text-gray-500">
          Free for all fire, EMS, hospital, and military agencies. EXOCHAIN
          production evidence verified; LiveSafe adapter claims remain gated.
        </p>
      </div>
    </div>
  );
}

export default AgencyRegister;
