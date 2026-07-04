import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

const AGENCY_TYPES = [
  { value: 'fire', label: 'Fire Department' },
  { value: 'ems', label: 'EMS / Ambulance' },
  { value: 'hospital', label: 'Hospital / ER Staff' },
  { value: 'police', label: 'Police / Law Enforcement' },
  { value: 'military', label: 'Military' },
];

function Register() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [agencyName, setAgencyName] = useState('');
  const [agencyType, setAgencyType] = useState('');
  const [role, setRole] = useState('');
  const [certification, setCertification] = useState('');
  const [isMilitary, setIsMilitary] = useState(false);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const { register } = useAuth();
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

    if (!email || !password) {
      setError('Email and password are required');
      return;
    }

    const passwordError = validatePassword(password);
    if (passwordError) {
      setError(passwordError);
      return;
    }

    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }

    if (!agencyName.trim()) {
      setError('Agency name is required');
      return;
    }

    if (!agencyType) {
      setError('Agency type is required');
      return;
    }

    if (!role.trim()) {
      setError('Your role/position is required');
      return;
    }

    setLoading(true);
    try {
      await register({
        email,
        password,
        agency_name: agencyName.trim(),
        agency_type: agencyType,
        role: role.trim(),
        certification: certification.trim() || undefined,
        is_military: isMilitary,
      });
      navigate('/dashboard');
    } catch (err) {
      if (err.response?.data?.error) {
        setError(err.response.data.error);
      } else {
        setError('Registration failed. Please try again.');
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
            First Responder Registration
          </p>
          <p className="mt-1 text-sm text-green-600 font-semibold">
            ✓ Free account for all first responders
          </p>
        </div>

        {/* Registration Form */}
        <div className="bg-white rounded-2xl shadow-lg p-8">
          <h2 className="text-2xl font-semibold text-gray-900 mb-6">Create Responder Account</h2>

          {error && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm" role="alert">
              {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Account Credentials */}
            <div className="border-b border-gray-200 pb-4 mb-4">
              <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Account Credentials</h3>
              <div className="space-y-3">
                <div>
                  <label htmlFor="email" className="block text-sm font-medium text-gray-700 mb-1">
                    Email Address <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="email"
                    name="email"
                    type="email"
                    autoComplete="email"
                    required
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                    placeholder="you@agency.gov"
                  />
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label htmlFor="password" className="block text-sm font-medium text-gray-700 mb-1">
                      Password <span className="text-red-500">*</span>
                    </label>
                    <input
                      id="password"
                      name="password"
                      type="password"
                      autoComplete="new-password"
                      required
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
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
                  Must contain uppercase, lowercase, and a number
                </p>
              </div>
            </div>

            {/* Agency Affiliation */}
            <div className="border-b border-gray-200 pb-4 mb-4">
              <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Agency Affiliation</h3>
              <div className="space-y-3">
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
                    placeholder="e.g. FDNY Station 42"
                  />
                </div>
              </div>
            </div>

            {/* Role & Certification */}
            <div>
              <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Role & Certification</h3>
              <div className="space-y-3">
                <div>
                  <label htmlFor="role" className="block text-sm font-medium text-gray-700 mb-1">
                    Your Role / Position <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="role"
                    name="role"
                    type="text"
                    required
                    value={role}
                    onChange={(e) => setRole(e.target.value)}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                    placeholder="e.g. Paramedic, Firefighter, ER Nurse"
                  />
                </div>

                <div>
                  <label htmlFor="certification" className="block text-sm font-medium text-gray-700 mb-1">
                    Certification / License #
                  </label>
                  <input
                    id="certification"
                    name="certification"
                    type="text"
                    value={certification}
                    onChange={(e) => setCertification(e.target.value)}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 transition text-base"
                    placeholder="e.g. EMT-P #12345"
                  />
                </div>

                <div className="flex items-center">
                  <input
                    id="isMilitary"
                    name="isMilitary"
                    type="checkbox"
                    checked={isMilitary}
                    onChange={(e) => setIsMilitary(e.target.checked)}
                    className="h-4 w-4 text-red-600 focus:ring-red-500 border-gray-300 rounded"
                  />
                  <label htmlFor="isMilitary" className="ml-2 block text-sm text-gray-700">
                    I am active military / veteran
                  </label>
                </div>
              </div>
            </div>

            <button
              type="submit"
              disabled={loading}
              className="w-full py-3 px-4 bg-red-500 hover:bg-red-600 disabled:bg-red-300 text-white font-semibold rounded-lg transition duration-200 text-base focus:ring-2 focus:ring-red-500 focus:ring-offset-2 mt-6"
            >
              {loading ? 'Creating Account...' : 'Register as First Responder'}
            </button>
          </form>

          <div className="mt-6 text-center">
            <p className="text-sm text-gray-600">
              Already have an account?{' '}
              <Link to="/login" className="text-red-600 hover:text-red-700 font-medium">
                Sign in
              </Link>
            </p>
          </div>
        </div>

        {/* Footer */}
        <p className="mt-6 text-center text-xs text-gray-500">
          Free for all fire, EMS, hospital, and military staff. EXOCHAIN
          production evidence verified; LiveSafe adapter claims remain gated.
        </p>
      </div>
    </div>
  );
}

export default Register;
