import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import api from '../services/api';

function TrusteeLogin() {
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
      const res = await api.post('/auth/trustee/login', { email, password });
      // Store trustee auth separately from subscriber auth
      localStorage.setItem('livesafe_trustee_token', res.data.token);
      localStorage.setItem('livesafe_trustee_user', JSON.stringify(res.data.user));
      navigate('/trustee/dashboard');
    } catch (err) {
      setError(err.response?.data?.error || 'Login failed');
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
          <p className="text-gray-600 mt-2">Sign in to your trustee dashboard</p>
        </div>

        <div className="space-y-6">
          <h2 className="text-xl font-semibold text-gray-900">Trustee Sign In</h2>

          {error && (
            <div className="p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" role="alert">
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
                placeholder="trustee@example.com"
                data-testid="trustee-login-email"
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
                data-testid="trustee-login-password"
              />
            </div>

            <button
              type="submit"
              disabled={submitting}
              className="w-full py-3 bg-sky-600 text-white font-medium rounded-lg hover:bg-sky-700 transition disabled:opacity-50"
              data-testid="trustee-login-btn"
            >
              {submitting ? 'Signing In...' : 'Sign In'}
            </button>
          </form>

          <p className="text-center text-sm text-gray-500">
            Not a trustee? <Link to="/login" className="text-sky-600 hover:text-sky-700 font-medium">Subscriber login</Link>
          </p>
        </div>

        <p className="text-center text-xs text-gray-400 mt-6">
          LiveSafe shows current P.A.C.E. role and access states inside the app.
        </p>
      </div>
    </div>
  );
}

export default TrusteeLogin;
