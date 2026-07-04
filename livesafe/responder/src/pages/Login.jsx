import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

function Login() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const { login } = useAuth();
  const navigate = useNavigate();

  const handleSubmit = async (e) => {
    e.preventDefault();
    setError('');

    if (!email || !password) {
      setError('Email and password are required');
      return;
    }

    setLoading(true);
    try {
      await login(email, password);
      navigate('/dashboard');
    } catch (err) {
      if (err.response?.data?.error) {
        setError(err.response.data.error);
      } else {
        setError('Login failed. Please try again.');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="min-h-screen bg-gray-900 flex items-center justify-center px-4 py-12"
      data-testid="high-contrast-login"
      style={{ fontSize: '18px' }}
    >
      <div className="w-full max-w-md">
        {/* Logo/Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-white">
            LiveSafe<span className="text-amber-300">.ai</span>
          </h1>
          <p className="mt-2 text-gray-300 text-lg">
            Responder Portal
          </p>
        </div>

        {/* Login Form — dark high-contrast card */}
        <div className="bg-gray-800 rounded-2xl border-2 border-gray-600 shadow-2xl p-8">
          <h2 className="text-2xl font-bold text-white mb-6">Sign In</h2>

          {error && (
            <div
              className="mb-4 p-4 bg-red-900 border-2 border-red-500 rounded-xl text-red-200 text-base font-semibold"
              role="alert"
            >
              ⚠️ {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-5">
            <div>
              <label htmlFor="email" className="block text-base font-bold text-gray-200 mb-2">
                Email Address
              </label>
              <input
                id="email"
                name="email"
                type="email"
                autoComplete="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full px-4 py-4 bg-gray-700 border-2 border-gray-500 rounded-xl text-white text-lg placeholder-gray-400 focus:border-red-400 focus:outline-none focus:ring-2 focus:ring-red-400 transition"
                style={{ minHeight: '56px' }}
                placeholder="you@agency.gov"
              />
            </div>

            <div>
              <label htmlFor="password" className="block text-base font-bold text-gray-200 mb-2">
                Password
              </label>
              <input
                id="password"
                name="password"
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full px-4 py-4 bg-gray-700 border-2 border-gray-500 rounded-xl text-white text-lg placeholder-gray-400 focus:border-red-400 focus:outline-none focus:ring-2 focus:ring-red-400 transition"
                style={{ minHeight: '56px' }}
                placeholder="Enter your password"
              />
            </div>

            <button
              type="submit"
              disabled={loading}
              className={`w-full py-4 px-4 rounded-xl text-white font-bold text-xl transition ${
                loading
                  ? 'bg-gray-600 cursor-wait text-gray-400'
                  : 'bg-red-800 hover:bg-red-700 active:bg-red-900'
              }`}
              style={{ minHeight: '60px' }}
            >
              {loading ? '⏳ Signing In...' : '🔐 Sign In'}
            </button>
          </form>

          <div className="mt-6 text-center">
            <p className="text-base text-gray-300">
              New first responder?{' '}
              <Link to="/register" className="text-amber-300 hover:text-amber-200 font-bold underline">
                Register here
              </Link>
            </p>
          </div>
        </div>

        <p className="mt-6 text-center text-sm text-gray-400">
          Free for all fire, EMS, hospital, and military staff. EXOCHAIN
          production evidence verified; LiveSafe adapter claims remain gated.
        </p>
      </div>
    </div>
  );
}

export default Login;
