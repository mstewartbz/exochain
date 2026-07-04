import React, { useState, useEffect } from 'react';
import { useSearchParams, useNavigate, Link } from 'react-router-dom';
import api from '../services/api';

function TrusteeAccept() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const token = searchParams.get('token') || searchParams.get('invitation_token');

  const [invitation, setInvitation] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState('');
  const [success, setSuccess] = useState(null);
  const [declined, setDeclined] = useState(false);
  const [declineMessage, setDeclineMessage] = useState('');
  const [showAcceptForm, setShowAcceptForm] = useState(false);
  const [isExpiredError, setIsExpiredError] = useState(false);
  const [resendStatus, setResendStatus] = useState(null); // null | 'requesting' | 'success' | 'error'
  const [resendMessage, setResendMessage] = useState('');

  useEffect(() => {
    if (!token) {
      setError('No invitation token provided');
      setLoading(false);
      return;
    }

    api.get(`/pace/invitation/${token}`)
      .then((res) => {
        setInvitation(res.data);
        setLoading(false);
      })
      .catch((err) => {
        const errorMsg = err.response?.data?.error || 'Invalid or expired invitation';
        setError(errorMsg);
        // Detect expired invitation specifically
        if (errorMsg.toLowerCase().includes('expired')) {
          setIsExpiredError(true);
        }
        setLoading(false);
      });
  }, [token]);

  const handleAccept = async (e) => {
    e.preventDefault();
    setSubmitError('');

    if (password.length < 6) {
      setSubmitError('Password must be at least 6 characters');
      return;
    }

    if (password !== confirmPassword) {
      setSubmitError('Passwords do not match');
      return;
    }

    setSubmitting(true);
    try {
      const res = await api.post('/pace/accept-invitation', {
        token,
        password,
        first_name: firstName || undefined,
        last_name: lastName || undefined,
      });

      setSuccess(res.data);
    } catch (err) {
      setSubmitError(err.response?.data?.error || 'Failed to create account');
    } finally {
      setSubmitting(false);
    }
  };

  const handleDecline = async () => {
    setSubmitError('');
    setSubmitting(true);
    try {
      const res = await api.post(`/pace/invitation/${token}/decline`);
      setDeclineMessage(res.data.message || 'You declined this P.A.C.E. invitation.');
      setDeclined(true);
    } catch (err) {
      setSubmitError(err.response?.data?.error || 'Failed to decline invitation');
    } finally {
      setSubmitting(false);
    }
  };

  const handleRequestNewInvitation = async () => {
    if (!token) return;
    setResendStatus('requesting');
    setResendMessage('');
    try {
      const res = await api.post(`/pace/invitation/${token}/request-resend`);
      setResendStatus('success');
      setResendMessage(res.data.message || 'A request has been sent to the subscriber to resend your invitation.');
    } catch (err) {
      setResendStatus('error');
      setResendMessage(err.response?.data?.error || 'Failed to request a new invitation. Please contact the subscriber directly.');
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Validating invitation...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="max-w-md w-full p-8 bg-white rounded-xl shadow-sm border border-gray-200 text-center">
          {isExpiredError ? (
            <>
              <div className="text-amber-500 text-4xl mb-4">⏰</div>
              <h2 className="text-xl font-bold text-gray-900 mb-2" data-testid="expired-invitation-title">
                Invitation Expired
              </h2>
              <p className="text-gray-600 mb-4" data-testid="invitation-error">
                This P.A.C.E. invitation has expired. Invitations are valid for 7 days from when they were sent.
              </p>
              <p className="text-sm text-gray-500 mb-6">
                To serve in this P.A.C.E. role, the subscriber will need to send you a new invitation.
              </p>

              {/* Request New Invitation */}
              {resendStatus === null && (
                <button
                  onClick={handleRequestNewInvitation}
                  className="w-full py-3 bg-sky-600 text-white font-medium rounded-lg hover:bg-sky-700 transition mb-4"
                  data-testid="request-new-invitation-btn"
                >
                  📧 Request New Invitation
                </button>
              )}

              {resendStatus === 'requesting' && (
                <div className="w-full py-3 bg-gray-100 text-gray-500 font-medium rounded-lg mb-4 flex items-center justify-center gap-2">
                  <div className="animate-spin h-4 w-4 border-2 border-sky-500 border-t-transparent rounded-full"></div>
                  Sending request...
                </div>
              )}

              {resendStatus === 'success' && (
                <div className="p-4 bg-green-50 border border-green-200 text-green-800 rounded-lg mb-4 text-sm" data-testid="resend-success">
                  ✅ {resendMessage}
                </div>
              )}

              {resendStatus === 'error' && (
                <div className="p-4 bg-red-50 border border-red-200 text-red-700 rounded-lg mb-4 text-sm" data-testid="resend-error">
                  ❌ {resendMessage}
                </div>
              )}

              <Link to="/" className="text-sky-600 hover:text-sky-700 font-medium text-sm">
                Go to Home
              </Link>
            </>
          ) : (
            <>
              <div className="text-red-500 text-4xl mb-4">&#10060;</div>
              <h2 className="text-xl font-bold text-gray-900 mb-2">Invalid Invitation</h2>
              <p className="text-gray-600 mb-6" data-testid="invitation-error">{error}</p>
              <Link to="/" className="text-sky-600 hover:text-sky-700 font-medium">
                Go to Home
              </Link>
            </>
          )}
        </div>
      </div>
    );
  }

  if (declined) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="max-w-md w-full p-8 bg-white rounded-xl shadow-sm border border-gray-200 text-center">
          <div className="text-gray-400 text-4xl mb-4">&#128532;</div>
          <h2 className="text-xl font-bold text-gray-900 mb-2">Invitation Declined</h2>
          <p className="text-gray-600 mb-4">
            {declineMessage || (
              <>
                You have declined the P.A.C.E. invitation from <span className="font-medium">{invitation.subscriber_name}</span>.
              </>
            )}
          </p>
          <p className="text-gray-500 text-sm mb-6">
            The subscriber can nominate a different person for this role.
          </p>
          <Link to="/" className="text-sky-600 hover:text-sky-700 font-medium">
            Go to Home
          </Link>
        </div>
      </div>
    );
  }

  if (success) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="max-w-md w-full p-8 bg-white rounded-xl shadow-sm border border-gray-200 text-center">
          <div className="text-emerald-500 text-4xl mb-4">&#9989;</div>
          <h2 className="text-xl font-bold text-gray-900 mb-2">P.A.C.E. Role Accepted</h2>
          <p className="text-gray-600 mb-2">
            You are now the <span className="font-medium">{invitation.role_name || invitation.role}</span> P.A.C.E. contact for{' '}
            <span className="font-medium">{invitation.subscriber_name}</span>.
          </p>
          <div className="my-4 p-3 bg-sky-50 rounded-lg border border-sky-200">
            <p className="text-sm text-sky-800">
              <span className="font-medium">Your DID:</span>{' '}
              <code className="text-xs bg-sky-100 px-2 py-0.5 rounded" data-testid="trustee-did">{success.user.did}</code>
            </p>
            <p className="text-sm text-sky-800 mt-1">
              <span className="font-medium">Role:</span>{' '}
              <span data-testid="trustee-role">{success.user.role}</span>
            </p>
            <p className="text-sm text-sky-800 mt-1">
              <span className="font-medium">Status:</span>{' '}
              <span className="text-emerald-600 font-medium" data-testid="trustee-status">accepted</span>
            </p>
            {success.user.shard_ref && (
              <p className="text-sm text-sky-800 mt-1">
                <span className="font-medium">Key Shard:</span>{' '}
                <code className="text-xs bg-sky-100 px-2 py-0.5 rounded" data-testid="trustee-shard-ref">{success.user.shard_ref}</code>
              </p>
            )}
          </div>
          <p className="text-sm text-gray-500 mb-4">
            {invitation.subscriber_name} has been notified that you accepted this role.
          </p>
          <Link to="/" className="text-sky-600 hover:text-sky-700 font-medium">
            Go to Home
          </Link>
        </div>
      </div>
    );
  }

  // Main invitation view with role explanation, accept/decline options
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12">
      <div className="max-w-lg w-full p-8 bg-white rounded-xl shadow-sm border border-gray-200">
        <div className="text-center mb-6">
          <h1 className="text-2xl font-bold text-sky-700">
            LiveSafe<span className="text-emerald-600">.ai</span>
          </h1>
          <h2 className="text-lg font-semibold text-gray-900 mt-4" data-testid="invitation-title">
            P.A.C.E. Invitation
          </h2>
          <p className="text-gray-600 mt-2" data-testid="invitation-subscriber">
            <span className="font-medium">{invitation.subscriber_name}</span> has invited you
            to be their <span className="font-medium capitalize">{invitation.role_name || invitation.role}</span> P.A.C.E. contact.
          </p>
        </div>

        <div className="mb-6 rounded-lg border border-teal-200 bg-teal-50 p-4" data-testid="invitee-boundaries">
          <p className="text-sm font-semibold text-teal-900">This is not a marketing invite.</p>
          <p className="mt-2 text-sm leading-6 text-teal-900">
            Accepting this role does not give you {invitation.subscriber_name}'s full medical records.
            You may only see information the subscriber explicitly shares with you or makes available for emergency purposes.
          </p>
          <p className="mt-2 text-sm leading-6 text-teal-900">
            You can accept, decline, or ask {invitation.subscriber_name} to choose someone else. You can also revoke later if your availability changes.
          </p>
        </div>

        {/* Role Info */}
        <div className="mb-6 p-4 bg-sky-50 rounded-lg border border-sky-200" data-testid="role-info">
          <div className="flex items-center gap-3 mb-3">
            <div className="w-10 h-10 rounded-full bg-sky-600 flex items-center justify-center text-white font-bold text-lg">
              {invitation.role_letter || invitation.role?.charAt(0)?.toUpperCase() || '?'}
            </div>
            <div>
              <h3 className="font-semibold text-sky-900">{invitation.role_name || invitation.role} Contact</h3>
              <p className="text-xs text-sky-700">P.A.C.E. Safety Circle role</p>
            </div>
          </div>
          <p className="text-sm text-sky-800 mb-3" data-testid="role-description">
            {invitation.role_description || 'You have been invited to serve as a P.A.C.E. contact.'}
          </p>
          {invitation.role_responsibilities && invitation.role_responsibilities.length > 0 && (
            <div>
              <h4 className="text-sm font-medium text-sky-900 mb-2">Role expectations:</h4>
              <ul className="space-y-1" data-testid="role-responsibilities">
                {invitation.role_responsibilities.map((resp, idx) => (
                  <li key={idx} className="text-sm text-sky-800 flex items-start gap-2">
                    <span className="text-sky-500 mt-0.5">&#8226;</span>
                    <span>{resp}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>

        {/* Invitation Details */}
        <div className="mb-6 p-3 bg-gray-50 rounded-lg border border-gray-200">
          <p className="text-sm text-gray-700">
            <span className="font-medium">Subscriber:</span>{' '}
            <span data-testid="subscriber-name">{invitation.subscriber_name}</span>
          </p>
          <p className="text-sm text-gray-700 mt-1">
            <span className="font-medium">Invited email:</span> {invitation.email}
          </p>
          <p className="text-sm text-gray-700 mt-1">
            <span className="font-medium">Role:</span> {invitation.role_name || invitation.role}
          </p>
        </div>

        {/* Accept/Decline Buttons */}
        {!showAcceptForm && (
          <div className="flex gap-3 mb-6" data-testid="accept-decline-options">
            <button
              onClick={() => setShowAcceptForm(true)}
              className="flex-1 py-3 bg-emerald-600 text-white font-medium rounded-lg hover:bg-emerald-700 transition"
              data-testid="accept-invitation-btn"
            >
              Accept Trusteeship
            </button>
            <button
              onClick={handleDecline}
              disabled={submitting}
              className="flex-1 py-3 bg-white text-gray-700 font-medium rounded-lg border border-gray-300 hover:bg-gray-50 transition"
              data-testid="decline-invitation-btn"
            >
              {submitting ? 'Declining...' : 'Decline'}
            </button>
          </div>
        )}
        {submitError && !showAcceptForm && (
          <div className="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700" role="alert">
            {submitError}
          </div>
        )}

        {/* Account Creation Form (shown after clicking Accept) */}
        {showAcceptForm && (
          <div>
            <h3 className="text-md font-semibold text-gray-900 mb-3">Create Your P.A.C.E. Account</h3>
            <form onSubmit={handleAccept} className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">First Name</label>
                  <input
                    type="text"
                    value={firstName}
                    onChange={(e) => setFirstName(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                    placeholder="First name"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Last Name</label>
                  <input
                    type="text"
                    value={lastName}
                    onChange={(e) => setLastName(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                    placeholder="Last name"
                  />
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Password</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  required
                  minLength={6}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                  placeholder="Create a password"
                  data-testid="trustee-password"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Confirm Password</label>
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  required
                  minLength={6}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-sky-500"
                  placeholder="Confirm password"
                  data-testid="trustee-confirm-password"
                />
              </div>

              {submitError && (
                <div className="p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm">
                  {submitError}
                </div>
              )}

              <div className="flex gap-3">
                <button
                  type="submit"
                  disabled={submitting}
                  className="flex-1 py-3 bg-emerald-600 text-white font-medium rounded-lg hover:bg-emerald-700 transition disabled:opacity-50"
                  data-testid="create-trustee-account-btn"
                >
                  {submitting ? 'Creating Account...' : 'Accept & Create Account'}
                </button>
                <button
                  type="button"
                  onClick={() => setShowAcceptForm(false)}
                  className="px-4 py-3 bg-white text-gray-700 font-medium rounded-lg border border-gray-300 hover:bg-gray-50 transition"
                >
                  Back
                </button>
              </div>
            </form>
          </div>
        )}

        {/* Link to create account */}
        {!showAcceptForm && (
          <p className="text-center text-sm text-gray-500" data-testid="create-account-link">
            By accepting, you will create a free LiveSafe.ai account to manage this P.A.C.E. role.
          </p>
        )}
      </div>
    </div>
  );
}

export default TrusteeAccept;
