import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import api from '../services/api';

function ProviderLogin() {
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e) => {
    e.preventDefault();
    setError('');
    setSubmitting(true);

    try {
      const res = await api.post('/auth/provider/login', { email, password });
      // Store provider auth in localStorage
      localStorage.setItem('livesafe_provider_token', res.data.token);
      localStorage.setItem('livesafe_provider_user', JSON.stringify(res.data.user));
      // Redirect to provider dashboard
      navigate('/provider/dashboard');
    } catch (err) {
      setError(err.response?.data?.error || 'Login failed. Please check your credentials.');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12">
      <div className="max-w-md w-full p-8 bg-white rounded-xl shadow-sm border border-gray-200">
        <div className="text-center mb-8">
          <h1 className="text-2xl font-bold text-sky-700">
            LiveSafe<span className="text-emerald-600">.ai</span>
          </h1>
          <p className="text-gray-600 mt-2">Sign in to your provider portal</p>
        </div>

        <div className="space-y-6">
          <h2 className="text-xl font-semibold text-gray-900">Provider Sign In</h2>

          {error && (
            <div className="p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" role="alert" data-testid="provider-login-error">
              {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Email Address</label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                placeholder="provider@hospital.com"
                data-testid="provider-login-email"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Password</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                placeholder="Enter your password"
                data-testid="provider-login-password"
              />
            </div>

            <button
              type="submit"
              disabled={submitting}
              className="w-full py-3 bg-sky-600 text-white font-medium rounded-lg hover:bg-sky-700 transition disabled:opacity-50"
              data-testid="provider-login-btn"
            >
              {submitting ? 'Signing In...' : 'Sign In'}
            </button>
          </form>

          <div className="text-center space-y-2 text-sm text-gray-500">
            <p>
              Not registered?{' '}
              <Link to="/provider/register" className="text-sky-700 hover:text-sky-800 font-medium">
                Register as a provider
              </Link>
            </p>
            <p>
              Not a provider?{' '}
              <Link to="/login" className="text-sky-700 hover:text-sky-800 font-medium">
                Subscriber login
              </Link>
            </p>
          </div>
        </div>

        <p className="text-center text-xs text-gray-400 mt-6">
          LiveSafe shows current account and access states inside the app.
        </p>
      </div>
    </div>
  );
}

export default ProviderLogin;
