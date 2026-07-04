import React, { useState, useRef } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

function Register() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  // Inline per-field validation state (Feature #298)
  const [isHero, setIsHero] = useState(false);

  // Inline per-field validation state (Feature #298)
  const [emailFieldError, setEmailFieldError] = useState('');
  const [passwordFieldError, setPasswordFieldError] = useState('');
  const [confirmFieldError, setConfirmFieldError] = useState('');

  const { register } = useAuth();
  const navigate = useNavigate();

  // Ref-based guard to prevent concurrent form submissions (Feature #193)
  // useRef is synchronously updated, unlike useState which is batched by React
  const isSubmittingRef = useRef(false);

  const validateEmail = (emailVal) => {
    // Must match: something@something.something (requires TLD)
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(emailVal)) return 'Please enter a valid email address (e.g. user@example.com)';
    return null;
  };

  const validatePassword = (pwd) => {
    if (pwd.length < 8) return 'Password must be at least 8 characters';
    if (!/[A-Z]/.test(pwd)) return 'Password must contain at least one uppercase letter';
    if (!/[a-z]/.test(pwd)) return 'Password must contain at least one lowercase letter';
    if (!/[0-9]/.test(pwd)) return 'Password must contain at least one number';
    return null;
  };

  const handleSubmit = async (e) => {
    e.preventDefault();

    // Prevent concurrent/double submissions — synchronous ref check
    if (isSubmittingRef.current) {
      return;
    }
    isSubmittingRef.current = true;

    setError('');

    // Validate
    if (!email || !password) {
      setError('Email and password are required');
      isSubmittingRef.current = false;
      return;
    }

    const emailError = validateEmail(email);
    if (emailError) {
      setError(emailError);
      isSubmittingRef.current = false;
      return;
    }

    const passwordError = validatePassword(password);
    if (passwordError) {
      setError(passwordError);
      isSubmittingRef.current = false;
      return;
    }

    if (password !== confirmPassword) {
      setError('Passwords do not match');
      isSubmittingRef.current = false;
      return;
    }

    setLoading(true);
    try {
      await register(email, password, firstName, lastName, isHero);
      navigate('/onboarding');
    } catch (err) {
      if (err.response?.data?.error) {
        setError(err.response.data.error);
      } else {
        setError('Registration failed. Please try again.');
      }
    } finally {
      setLoading(false);
      isSubmittingRef.current = false;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-sky-50 to-white flex items-center justify-center px-4 py-12">
      <div className="w-full max-w-md">
        {/* Logo/Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-sky-700">
            LiveSafe<span className="text-emerald-600">.ai</span>
          </h1>
          <p className="mt-2 text-gray-600">
            Create your card, then invite your P.A.C.E. Safety Circle
          </p>
        </div>

        {/* Registration Form */}
        <div className="bg-white rounded-2xl shadow-lg p-8">
          <h2 className="text-2xl font-semibold text-gray-900 mb-2">Start your Safety Circle</h2>
          <p className="mb-6 text-sm text-gray-600">
            After account creation, LiveSafe opens the guided card and invitation setup.
          </p>

          {error && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm" role="alert">
              {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4" noValidate>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label htmlFor="firstName" className="block text-sm font-medium text-gray-700 mb-1">
                  First Name
                </label>
                <input
                  id="firstName"
                  name="firstName"
                  type="text"
                  autoComplete="given-name"
                  value={firstName}
                  onChange={(e) => setFirstName(e.target.value)}
                  className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-sky-500 focus:border-sky-500 transition text-base"
                  placeholder="John"
                />
              </div>
              <div>
                <label htmlFor="lastName" className="block text-sm font-medium text-gray-700 mb-1">
                  Last Name
                </label>
                <input
                  id="lastName"
                  name="lastName"
                  type="text"
                  autoComplete="family-name"
                  value={lastName}
                  onChange={(e) => setLastName(e.target.value)}
                  className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-sky-500 focus:border-sky-500 transition text-base"
                  placeholder="Doe"
                />
              </div>
            </div>

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
                onChange={(e) => {
                  const val = e.target.value;
                  setEmail(val);
                  // Inline real-time validation (Feature #298): validate as user types
                  if (val.length > 0) {
                    const err = validateEmail(val);
                    setEmailFieldError(err || '');
                  } else {
                    setEmailFieldError('');
                  }
                }}
                className={`w-full px-4 py-3 border rounded-lg focus:ring-2 focus:ring-sky-500 focus:border-sky-500 transition text-base ${
                  emailFieldError ? 'border-red-400 bg-red-50' : 'border-gray-300'
                }`}
                placeholder="you@example.com"
                data-testid="email-input"
                aria-describedby={emailFieldError ? 'email-field-error' : undefined}
                aria-invalid={!!emailFieldError}
              />
              {emailFieldError && (
                <p
                  id="email-field-error"
                  data-testid="email-field-error"
                  className="mt-1 text-xs text-red-600 flex items-center gap-1"
                  role="alert"
                >
                  <span>⚠</span> {emailFieldError}
                </p>
              )}
            </div>

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
                onChange={(e) => {
                  const val = e.target.value;
                  setPassword(val);
                  // Inline real-time validation (Feature #298): validate as user types
                  if (val.length > 0) {
                    const err = validatePassword(val);
                    setPasswordFieldError(err || '');
                  } else {
                    setPasswordFieldError('');
                  }
                  // Re-validate confirm password if it has a value
                  if (confirmPassword.length > 0) {
                    setConfirmFieldError(val !== confirmPassword ? 'Passwords do not match' : '');
                  }
                }}
                className={`w-full px-4 py-3 border rounded-lg focus:ring-2 focus:ring-sky-500 focus:border-sky-500 transition text-base ${
                  passwordFieldError ? 'border-red-400 bg-red-50' : 'border-gray-300'
                }`}
                placeholder="Min 8 characters"
                data-testid="password-input"
                aria-describedby={passwordFieldError ? 'password-field-error' : undefined}
                aria-invalid={!!passwordFieldError}
              />
              {passwordFieldError ? (
                <p
                  id="password-field-error"
                  data-testid="password-field-error"
                  className="mt-1 text-xs text-red-600 flex items-center gap-1"
                  role="alert"
                >
                  <span>⚠</span> {passwordFieldError}
                </p>
              ) : (
                <p className="mt-1 text-xs text-gray-500">
                  Must contain uppercase, lowercase, and a number
                </p>
              )}
            </div>

            <div>
              <label htmlFor="confirmPassword" className="block text-sm font-medium text-gray-700 mb-1">
                Confirm Password <span className="text-red-500">*</span>
              </label>
              <input
                id="confirmPassword"
                name="confirmPassword"
                type="password"
                autoComplete="new-password"
                required
                value={confirmPassword}
                onChange={(e) => {
                  const val = e.target.value;
                  setConfirmPassword(val);
                  // Inline real-time validation (Feature #298): validate as user types
                  if (val.length > 0) {
                    setConfirmFieldError(val !== password ? 'Passwords do not match' : '');
                  } else {
                    setConfirmFieldError('');
                  }
                }}
                className={`w-full px-4 py-3 border rounded-lg focus:ring-2 focus:ring-sky-500 focus:border-sky-500 transition text-base ${
                  confirmFieldError ? 'border-red-400 bg-red-50' : 'border-gray-300'
                }`}
                placeholder="Repeat your password"
                data-testid="confirm-password-input"
                aria-describedby={confirmFieldError ? 'confirm-field-error' : undefined}
                aria-invalid={!!confirmFieldError}
              />
              {confirmFieldError && (
                <p
                  id="confirm-field-error"
                  data-testid="confirm-field-error"
                  className="mt-1 text-xs text-red-600 flex items-center gap-1"
                  role="alert"
                >
                  <span>⚠</span> {confirmFieldError}
                </p>
              )}
            </div>

            {/* Heroes Free Tier */}
            <div className="p-3 bg-emerald-50 border border-emerald-200 rounded-lg" data-testid="heroes-free-tier-section">
              <label className="flex items-start gap-3 cursor-pointer" htmlFor="is-hero-checkbox">
                <input
                  id="is-hero-checkbox"
                  type="checkbox"
                  checked={isHero}
                  onChange={(e) => setIsHero(e.target.checked)}
                  className="mt-0.5 h-4 w-4 text-emerald-600 border-gray-300 rounded focus:ring-emerald-500"
                  data-testid="heroes-free-tier-checkbox"
                />
                <div>
                  <span className="text-sm font-medium text-emerald-800">
                    I am a Hero: first responder, law enforcement, Fire & Rescue, ER, FEMA/NIMS, powerline, military, or veteran
                  </span>
                  <p className="text-xs text-emerald-700 mt-0.5">
                    Heroes accounts are <strong>free forever</strong>: no payment required, no expiration.
                  </p>
                </div>
              </label>
              {isHero && (
                <div className="mt-2 flex items-center gap-2 text-xs text-emerald-700 font-medium" data-testid="heroes-free-tier-confirmed">
                  <span>✓</span> Your account will be registered as a free Heroes account
                </div>
              )}
            </div>

            {/* Submit button is disabled while loading to prevent double-submission (Feature #193) */}
            <button
              type="submit"
              disabled={loading}
              data-testid="register-submit-btn"
              aria-busy={loading}
              className="w-full py-3 px-4 bg-sky-500 hover:bg-sky-600 disabled:bg-sky-300 disabled:cursor-not-allowed text-white font-semibold rounded-lg transition duration-200 text-base focus:ring-2 focus:ring-sky-500 focus:ring-offset-2"
            >
              {loading ? 'Creating Account...' : 'Create Account'}
            </button>

            {loading && (
              <p className="text-center text-xs text-gray-500" data-testid="submitting-indicator">
                Processing your registration...
              </p>
            )}
          </form>

          <div className="mt-6 text-center">
            <p className="text-sm text-gray-600">
              Already have an account?{' '}
              <Link to="/login" className="text-sky-700 hover:text-sky-800 font-medium">
                Sign in
              </Link>
            </p>
          </div>
        </div>

        {/* Footer */}
        <p className="mt-6 text-center text-xs text-gray-500">
          LiveSafe shows current setup and invitation states inside the app.
        </p>
      </div>
    </div>
  );
}

export default Register;
