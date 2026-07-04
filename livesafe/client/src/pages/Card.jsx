import React, { useState, useEffect, useRef } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate } from 'react-router-dom';
import api from '../services/api';
import Navbar from '../components/Navbar';

function Card() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [cardData, setCardData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [issuing, setIssuing] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [blockedMessage, setBlockedMessage] = useState('');
  const [nfcData, setNfcData] = useState(null);
  const [showOrderModal, setShowOrderModal] = useState(false);
  // Feature #270: Idempotency guard - prevents double-click from creating duplicate card requests
  // useRef is synchronous unlike useState, so it blocks re-entry even before React re-renders
  const issuingRef = useRef(false);

  const fetchCardStatus = async () => {
    try {
      setLoading(true);
      const res = await api.get('/card/me');
      setCardData(res.data);
      // Fetch NFC payload if user has a DID
      if (user?.did) {
        try {
          const nfcRes = await api.get('/card/' + user.did + '/nfc');
          setNfcData(nfcRes.data);
        } catch (nfcErr) {
          console.error('Failed to fetch NFC payload:', nfcErr);
        }
      }
    } catch (err) {
      console.error('Failed to fetch card status:', err);
      setError('Failed to load card status');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCardStatus();
  }, []);

  const handleIssueCard = async () => {
    // Feature #270: Idempotency guard - useRef is synchronous, prevents double-click race condition
    if (issuingRef.current) return;
    issuingRef.current = true;
    try {
      setIssuing(true);
      setError('');
      setBlockedMessage('');
      setSuccess('');
      const res = await api.post('/card/issue');
      if (res.data.already_issued) {
        // Idempotent: card already exists, no duplicate created
        setSuccess('Your emergency card is already issued. No duplicate created.');
      } else {
        setSuccess(res.data.message || 'Emergency card issued successfully!');
      }
      fetchCardStatus();
    } catch (err) {
      if (err.response?.status === 403 && err.response?.data?.blocked) {
        setBlockedMessage(err.response.data.message);
      } else {
        setError(err.response?.data?.error || 'Failed to issue card');
      }
    } finally {
      setIssuing(false);
      issuingRef.current = false;
    }
  };

  const handleRetryIssueCard = async () => {
    // Refresh card status first to pick up any prerequisite changes (e.g., trustees now accepted)
    setError('');
    setBlockedMessage('');
    setSuccess('');
    setIssuing(true);
    try {
      // Re-fetch status to get latest can_issue state
      const statusRes = await api.get('/card/me');
      setCardData(statusRes.data);
      // Attempt card issuance
      const res = await api.post('/card/issue');
      setSuccess(res.data.message || 'Emergency card issued successfully!');
      fetchCardStatus();
    } catch (err) {
      if (err.response?.status === 403 && err.response?.data?.blocked) {
        setBlockedMessage(err.response.data.message);
      } else {
        setError(err.response?.data?.error || 'Failed to issue card. Please ensure all prerequisites are met.');
      }
    } finally {
      setIssuing(false);
    }
  };

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const handleDownloadPdf = (format) => {
    if (!user?.did) return;
    // Open in new tab for download
    const url = '/api/card/' + encodeURIComponent(user.did) + '/pdf?format=' + format;
    window.open(url, '_blank');
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main className="max-w-2xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {loading ? (
          <div className="flex items-center justify-center py-20">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
              <p className="text-gray-600">Loading card status...</p>
            </div>
          </div>
        ) : (
          <div className="space-y-6">
            {/* PACE Status */}
            <div className="bg-white rounded-xl shadow-sm border p-6" data-testid="pace-gate-status">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-gray-900">P.A.C.E. Safety Circle</h2>
                <span className={`text-xl font-bold ${cardData?.pace_complete ? 'text-emerald-600' : 'text-amber-600'}`} data-testid="pace-trustees-count">
                  {cardData?.accepted_trustees ?? 0}/4
                </span>
              </div>
              <p className="text-sm text-gray-500 mb-3">
                All 4 P.A.C.E. contacts (Primary, Alternate, Contingent, Emergency) must accept their invitations before you can issue your emergency card.
              </p>
              {cardData?.pace_complete ? (
                <div className="p-3 bg-emerald-50 border border-emerald-200 rounded-lg" data-testid="pace-gate-passed">
                  <p className="text-sm text-emerald-800 font-medium">All 4 P.A.C.E. contacts have accepted. Safety Circle requirement met.</p>
                </div>
              ) : (
                <div className="p-3 bg-amber-50 border border-amber-200 rounded-lg" data-testid="pace-gate-warning">
                  <p className="text-sm text-amber-800">
                    <strong>Card issuance blocked:</strong> {cardData?.accepted_trustees ?? 0} of 4 P.A.C.E. contacts have accepted.
                    Go to <a href="/pace" className="underline text-amber-700 hover:text-amber-900">P.A.C.E. Safety Circle</a> to invite people by Email, SMS, or Copy link.
                  </p>
                </div>
              )}
            </div>

            {/* Score Status */}
            <div className="bg-white rounded-xl shadow-sm border p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-gray-900">Identity Core Score</h2>
                <span className="text-2xl font-bold text-sky-700" data-testid="identity-core-score">
                  {cardData?.identity_core_score || 0}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-sm text-gray-500">Minimum required for card issuance:</span>
                <span className="text-sm font-semibold text-gray-700">{cardData?.minimum_score || 10}</span>
              </div>
              {!cardData?.can_issue && !cardData?.card && (cardData?.identity_core_score ?? 0) < (cardData?.minimum_score ?? 10) && (
                <div className="mt-3 p-3 bg-amber-50 border border-amber-200 rounded-lg" data-testid="score-gate-warning">
                  <p className="text-sm text-amber-800">
                    Your identity core score ({cardData?.identity_core_score || 0}) is below the minimum ({cardData?.minimum_score || 10}).
                    Verify your email to increase your Core Identity score.
                  </p>
                </div>
              )}
              {cardData?.pace_complete && (cardData?.identity_core_score ?? 0) >= (cardData?.minimum_score ?? 10) && !cardData?.card && (
                <div className="mt-3 p-3 bg-green-50 border border-green-200 rounded-lg" data-testid="score-gate-passed">
                  <p className="text-sm text-green-800">
                    Your score meets the minimum requirement. You can issue your emergency card!
                  </p>
                </div>
              )}
            </div>

            {/* Card Display or Issue Button */}
            {cardData?.card ? (
              <div className="bg-white rounded-xl shadow-sm border p-6">
                <div className="text-center mb-4">
                  <span className="inline-flex items-center px-3 py-1 rounded-full text-sm font-semibold bg-green-100 text-green-800">
                    Card Issued
                  </span>
                </div>
                <h3 className="text-lg font-semibold text-gray-900 text-center mb-4">Your Emergency Card</h3>
                {cardData.card.qr_image_url && (
                  <div className="flex justify-center mb-4">
                    <img
                      src={cardData.card.qr_image_url}
                      alt="Emergency QR Code"
                      className="w-48 h-48 border-2 border-gray-200 rounded-lg"
                      data-testid="card-qr-code"
                    />
                  </div>
                )}
                <div className="text-center text-sm text-gray-600 space-y-1 mb-4">
                  <p><span className="font-medium">DID:</span> <span data-testid="card-did">{user?.did}</span></p>
                  <p><span className="font-medium">Status:</span> {cardData.card.status}</p>
                  <p><span className="font-medium">Issued:</span> {new Date(cardData.card.issued_at).toLocaleDateString()}</p>
                  {cardData.card.expires_at && (
                    <p><span className="font-medium">Expires:</span> {new Date(cardData.card.expires_at).toLocaleDateString()}</p>
                  )}
                </div>
                {/* QR Code contents info */}
                <div className="mt-2 p-3 bg-sky-50 border border-sky-200 rounded-lg text-left" data-testid="qr-contents-info">
                  <p className="text-xs font-semibold text-sky-800 mb-1">QR Code exposes pointer-only metadata:</p>
                  <ul className="text-xs text-sky-700 space-y-0.5 list-disc list-inside">
                    <li>Subscriber DID: <span data-testid="qr-encoded-did" className="font-mono text-sky-900 break-all">{user?.did}</span></li>
                    <li>Pointer state: <span data-testid="qr-pointer-state" className="font-medium text-sky-900">{cardData.card.qr_pointer_state}</span></li>
                    <li>Access type: <code className="text-sky-900">emergency_access</code></li>
                  </ul>
                  <p className="mt-2 pt-2 border-t border-sky-200 text-xs text-sky-700">
                    Pointer-only metadata is shown here. Tokenized responder routes remain redacted from account JSON.
                  </p>
                </div>

                {/* NFC Payload Info */}
                {nfcData && nfcData.nfc_payload && (
                  <div className="mt-4 p-3 bg-purple-50 border border-purple-200 rounded-lg text-left" data-testid="nfc-payload-info">
                    <p className="text-xs font-semibold text-purple-800 mb-1">NFC Tap Payload (metadata-only):</p>
                    <ul className="text-xs text-purple-700 space-y-0.5 list-disc list-inside">
                      <li>DID: <span data-testid="nfc-did" className="font-mono text-purple-900 break-all">{nfcData.nfc_payload.did}</span></li>
                      <li>Pointer state: <span data-testid="nfc-pointer-state" className="font-medium text-purple-900">{nfcData.pointer_state || 'metadata-only'}</span></li>
                      <li>Access type: <code className="text-purple-900">{nfcData.nfc_payload.type}</code></li>
                    </ul>
                    {nfcData.matches_qr && (
                      <p className="mt-1 text-xs text-purple-600 font-medium">✓ NFC payload matches QR code data</p>
                    )}
                  </div>
                )}

                {/* PDF Download Section */}
                <div className="mt-4 p-4 bg-gray-50 border border-gray-200 rounded-lg text-left" data-testid="pdf-download-section">
                  <p className="text-sm font-semibold text-gray-800 mb-3">Download Card PDF</p>
                  <div className="flex flex-col sm:flex-row gap-2">
                    <button
                      onClick={() => handleDownloadPdf('wallet')}
                      className="flex-1 px-4 py-2 bg-sky-600 hover:bg-sky-700 text-white text-sm font-medium rounded-lg transition"
                      data-testid="download-wallet-pdf-btn"
                    >
                      📄 Wallet Size (3.5" × 2")
                    </button>
                    <button
                      onClick={() => handleDownloadPdf('sticker')}
                      className="flex-1 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-medium rounded-lg transition"
                      data-testid="download-sticker-pdf-btn"
                    >
                      🏷️ Sticker Format (2" × 2")
                    </button>
                    <button
                      onClick={() => handleDownloadPdf('a4')}
                      className="flex-1 px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white text-sm font-medium rounded-lg transition"
                      data-testid="download-a4-pdf-btn"
                    >
                      📋 Full Page (A4)
                    </button>
                  </div>
                  <p className="mt-2 text-xs text-gray-500">All formats include QR code for emergency access</p>
                </div>

                {/* Scan History Link */}
                <div className="mt-4 p-4 bg-sky-50 border border-sky-200 rounded-lg text-left" data-testid="scan-history-section">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-semibold text-sky-800 mb-1">📱 Scan History</p>
                      <p className="text-xs text-sky-700">View all emergency card scans by first responders. Each scan records a local audit receipt; EXOCHAIN anchoring remains pending adapter verification.</p>
                    </div>
                    <button
                      onClick={() => navigate('/scan-history')}
                      className="ml-4 px-4 py-2 bg-sky-600 hover:bg-sky-700 text-white text-sm font-medium rounded-lg transition flex-shrink-0"
                      data-testid="view-scan-history-btn"
                    >
                      View History
                    </button>
                  </div>
                </div>

                {/* Order Physical Card Section (card issued state) */}
                <div className="mt-4 p-4 bg-amber-50 border border-amber-200 rounded-lg text-left" data-testid="order-physical-card-section">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-semibold text-amber-800 mb-1">💳 Order Physical Card</p>
                      <p className="text-xs text-amber-700">Get a durable physical card with your QR code for your wallet, medical bracelet, or ID holder.</p>
                    </div>
                    <button
                      onClick={() => setShowOrderModal(true)}
                      className="ml-4 px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white text-sm font-medium rounded-lg transition flex-shrink-0"
                      data-testid="order-physical-card-btn"
                    >
                      Order Card
                    </button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="bg-white rounded-xl shadow-sm border p-6 text-center">
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Issue Your Emergency Card</h3>
                <p className="text-sm text-gray-600 mb-6">
                  Your QR/NFC emergency card allows first responders to access your critical health data.
                </p>

                {/* Error Messages */}
                {error && (
                  <div className="mb-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm" data-testid="card-error-message">
                    <p className="font-medium mb-1">Card Generation Failed</p>
                    <p>{error}</p>
                  </div>
                )}
                {blockedMessage && (
                  <div className="mb-4 p-3 bg-amber-50 border border-amber-200 text-amber-700 rounded-lg text-sm" data-testid="blocked-message">
                    <p className="font-medium mb-1">Card Issuance Blocked</p>
                    <p>{blockedMessage}</p>
                  </div>
                )}
                {success && (
                  <div className="mb-4 p-3 bg-green-50 border border-green-200 text-green-700 rounded-lg text-sm">
                    {success}
                  </div>
                )}

                {/* Loading spinner shown during card generation (Feature #291) */}
                {issuing && (
                  <div
                    className="flex items-center justify-center gap-3 py-3 mb-3 bg-sky-50 border border-sky-200 rounded-lg"
                    data-testid="card-generation-spinner"
                  >
                    <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-sky-600 flex-shrink-0"></div>
                    <span className="text-sm text-sky-700 font-medium">Generating your emergency card…</span>
                  </div>
                )}

                <button
                  onClick={handleIssueCard}
                  disabled={issuing}
                  className={`px-8 py-3 rounded-lg text-white font-semibold text-sm transition ${
                    cardData?.can_issue
                      ? 'bg-sky-600 hover:bg-sky-700'
                      : 'bg-gray-500 hover:bg-gray-600'
                  } disabled:opacity-50 disabled:cursor-not-allowed`}
                  data-testid="issue-card-btn"
                  title={!cardData?.pace_complete ? '4 P.A.C.E. contacts must accept before card issuance' : !cardData?.can_issue ? 'Identity score requirement not met' : ''}
                >
                  {issuing ? (
                    <span className="flex items-center gap-2">
                      <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white inline-block"></span>
                      Issuing Card…
                    </span>
                  ) : 'Issue Emergency Card'}
                </button>

                {/* Retry button — shown when there's an error, allows retrying after fixing prerequisites */}
                {(error || blockedMessage) && (
                  <button
                    onClick={handleRetryIssueCard}
                    disabled={issuing}
                    className="mt-3 px-8 py-3 rounded-lg text-white font-semibold text-sm transition bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed"
                    data-testid="retry-card-btn"
                  >
                    {issuing ? 'Retrying...' : '🔄 Retry Card Generation'}
                  </button>
                )}

                {!cardData?.pace_complete && (
                  <p className="mt-3 text-xs text-gray-500" data-testid="pace-required-hint">
                    Card issuance requires all 4 P.A.C.E. contacts to accept ({cardData?.accepted_trustees ?? 0}/4 accepted)
                  </p>
                )}
                {cardData?.pace_complete && !cardData?.can_issue && (
                  <p className="mt-3 text-xs text-gray-500">
                    Card issuance requires a minimum 0dentity score of {cardData?.minimum_score || 10}
                  </p>
                )}

                {/* Order Physical Card - also shown when card not yet issued */}
                <div className="mt-6 pt-4 border-t border-gray-200 text-left" data-testid="order-physical-card-section">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-semibold text-amber-800 mb-1">💳 Order Physical Card</p>
                      <p className="text-xs text-amber-700">Get a durable physical card with your QR code for your wallet or ID holder.</p>
                    </div>
                    <button
                      onClick={() => setShowOrderModal(true)}
                      className="ml-4 px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white text-sm font-medium rounded-lg transition flex-shrink-0"
                      data-testid="order-physical-card-btn"
                    >
                      Order Card
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </main>

      {/* Order Physical Card Modal */}
      {showOrderModal && (
        <div
          className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center px-4 z-50"
          data-testid="order-card-modal"
          role="dialog"
          aria-modal="true"
          aria-labelledby="order-card-modal-title"
        >
          <div className="bg-white rounded-2xl shadow-xl p-8 max-w-md w-full">
            <div className="text-center mb-4">
              <div className="text-5xl mb-3">🚧</div>
              <h2 className="text-xl font-bold text-gray-900" id="order-card-modal-title" data-testid="order-card-modal-title">
                Physical Card Ordering
              </h2>
              <p className="text-sm text-gray-500 mt-1">Coming Soon</p>
            </div>
            <div className="bg-amber-50 border border-amber-200 rounded-lg p-4 mb-6" data-testid="order-card-coming-soon-message">
              <p className="text-sm text-amber-800 font-medium mb-2">🎯 Vendor integration in progress</p>
              <p className="text-sm text-amber-700">
                Physical card ordering will be available soon. Your durable card will include:
              </p>
              <ul className="mt-2 text-sm text-amber-700 space-y-1 list-disc list-inside">
                <li>Printed QR code for emergency access</li>
                <li>NFC chip with your identity payload</li>
                <li>Medical alert information</li>
                <li>Wallet-sized format (3.5" × 2")</li>
              </ul>
            </div>
            <div className="space-y-3">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Email address for order notification
                </label>
                <input
                  type="email"
                  placeholder="your@email.com"
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-amber-500 focus:border-amber-500"
                  data-testid="order-card-email-input"
                  defaultValue={user?.email || ''}
                />
              </div>
              <button
                onClick={() => {
                  alert('You have been added to the waitlist! We will notify you when physical card ordering is available.');
                  setShowOrderModal(false);
                }}
                className="w-full py-2.5 px-4 bg-amber-600 hover:bg-amber-700 text-white font-semibold rounded-lg transition"
                data-testid="order-card-notify-btn"
              >
                Notify Me When Available
              </button>
              <button
                onClick={() => setShowOrderModal(false)}
                className="w-full py-2.5 px-4 bg-gray-100 hover:bg-gray-200 text-gray-700 font-medium rounded-lg transition"
                data-testid="order-card-close-btn"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default Card;
