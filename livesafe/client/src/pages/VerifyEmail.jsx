import React, { useState, useEffect, useRef } from 'react';
import { useSearchParams, Link } from 'react-router-dom';
import api from '../services/api';

export default function VerifyEmail() {
  const [searchParams] = useSearchParams();
  const token = searchParams.get('token');
  const [status, setStatus] = useState('loading'); // loading, success, already_verified, error
  const [verificationTarget, setVerificationTarget] = useState('');
  const [errorMessage, setErrorMessage] = useState('');
  const calledRef = useRef(false);

  useEffect(() => {
    if (calledRef.current) return;
    calledRef.current = true;

    if (!token) {
      setStatus('error');
      setErrorMessage('No verification token provided');
      return;
    }

    const verifyEmail = async () => {
      try {
        const response = await api.get(`/auth/verify-email?token=${encodeURIComponent(token)}`);
        if (response.data.already_verified) {
          setStatus('already_verified');
        } else {
          setStatus('success');
        }
        setVerificationTarget(response.data.verification_target || '');
      } catch (err) {
        setStatus('error');
        setErrorMessage(
          err.response?.data?.error || 'Verification failed. The link may be invalid or expired.'
        );
      }
    };

    verifyEmail();
  }, [token]);

  return (
    <div className="min-h-screen bg-gradient-to-b from-sky-50 to-white flex items-center justify-center px-4">
      <div className="max-w-md w-full bg-white rounded-lg shadow-lg p-8 text-center">
        <div className="mb-6">
          <h1 className="text-2xl font-bold text-gray-900 mb-2">Email Verification</h1>
        </div>

        {status === 'loading' && (
          <div>
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
            <p className="text-gray-600">Verifying your email...</p>
          </div>
        )}

        {status === 'success' && (
          <div>
            <div className="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto mb-4">
              <svg className="w-8 h-8 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <h2 className="text-xl font-semibold text-green-700 mb-2">Email Verified!</h2>
            <p className="text-gray-600 mb-6">
              {verificationTarget ? (
                <>Your email <strong>{verificationTarget}</strong> has been verified successfully.</>
              ) : (
                'Your email has been verified successfully.'
              )}
            </p>
            <Link
              to="/login"
              className="inline-block bg-sky-500 hover:bg-sky-600 text-white font-medium py-2 px-6 rounded-lg transition-colors"
            >
              Sign In
            </Link>
          </div>
        )}

        {status === 'already_verified' && (
          <div>
            <div className="w-16 h-16 bg-blue-100 rounded-full flex items-center justify-center mx-auto mb-4">
              <svg className="w-8 h-8 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <h2 className="text-xl font-semibold text-blue-700 mb-2">Already Verified</h2>
            <p className="text-gray-600 mb-6">
              {verificationTarget ? (
                <>Your email <strong>{verificationTarget}</strong> has already been verified.</>
              ) : (
                'Your email has already been verified.'
              )}
            </p>
            <Link
              to="/login"
              className="inline-block bg-sky-500 hover:bg-sky-600 text-white font-medium py-2 px-6 rounded-lg transition-colors"
            >
              Sign In
            </Link>
          </div>
        )}

        {status === 'error' && (
          <div>
            <div className="w-16 h-16 bg-red-100 rounded-full flex items-center justify-center mx-auto mb-4">
              <svg className="w-8 h-8 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </div>
            <h2 className="text-xl font-semibold text-red-700 mb-2">Verification Failed</h2>
            <p className="text-gray-600 mb-6">{errorMessage}</p>
            <Link
              to="/register"
              className="inline-block bg-sky-500 hover:bg-sky-600 text-white font-medium py-2 px-6 rounded-lg transition-colors"
            >
              Register Again
            </Link>
          </div>
        )}
      </div>
    </div>
  );
}
