import React, { useEffect, useMemo, useState } from 'react';
import { useAuth } from '../context/AuthContext';
import Navbar from '../components/Navbar';
import api from '../services/api';
import { PACE_ROLES, normalizePaceRole, sortPaceItems } from '../lib/paceRoles';

const CHANNELS = [
  { key: 'email', label: 'Email' },
  { key: 'sms', label: 'SMS' },
  { key: 'link', label: 'Copy link' },
];

const REPLACE_MODE_KEY = 'pace_replace_mode';
const REPLACE_EMAIL_KEY = 'pace_replace_email';

function loadReplaceState() {
  try {
    const mode = sessionStorage.getItem(REPLACE_MODE_KEY);
    const email = sessionStorage.getItem(REPLACE_EMAIL_KEY) || '';
    return { mode: mode ? Number(mode) || null : null, email };
  } catch {
    return { mode: null, email: '' };
  }
}

function saveReplaceState(mode, email) {
  try {
    if (mode) {
      sessionStorage.setItem(REPLACE_MODE_KEY, String(mode));
      sessionStorage.setItem(REPLACE_EMAIL_KEY, email || '');
    } else {
      sessionStorage.removeItem(REPLACE_MODE_KEY);
      sessionStorage.removeItem(REPLACE_EMAIL_KEY);
    }
  } catch {
    /* session persistence is best-effort */
  }
}

function defaultChannels() {
  return { email: true, sms: false, link: true };
}

function selectedChannels(channels) {
  return Object.entries(channels)
    .filter(([, enabled]) => enabled)
    .map(([channel]) => channel);
}

function deliverySummary(trustee) {
  const email = trustee.email_delivery_status || 'not_requested';
  const sms = trustee.sms_delivery_status || 'not_requested';
  const link = trustee.invitation_url ? 'available' : 'not_ready';
  return `Email: ${email}. SMS: ${sms}. Copy link: ${link}.`;
}

function statusLabel(status) {
  if (status === 'accepted') return 'Accepted';
  if (status === 'declined') return 'Declined';
  if (status === 'replaced') return 'Replaced';
  return 'Pending';
}

export default function Pace() {
  const { user } = useAuth();
  const [trustees, setTrustees] = useState([]);
  const [vssCeremony, setVssCeremony] = useState(null);
  const [loading, setLoading] = useState(true);
  const [notice, setNotice] = useState('');
  const [error, setError] = useState('');
  const [activeRole, setActiveRole] = useState(null);
  const [inviteDraft, setInviteDraft] = useState({ email: '', phone: '', channels: defaultChannels() });
  const [sending, setSending] = useState(false);
  const [copyStatus, setCopyStatus] = useState('');
  const [replaceMode, setReplaceMode] = useState(loadReplaceState().mode);
  const [replaceEmail, setReplaceEmail] = useState(loadReplaceState().email);
  const [replaceSubmitting, setReplaceSubmitting] = useState(false);

  const normalizedTrustees = useMemo(
    () => sortPaceItems(trustees.map((trustee) => ({
      ...trustee,
      role: normalizePaceRole(trustee.role),
    }))),
    [trustees],
  );

  const acceptedCount = normalizedTrustees.filter((trustee) => trustee.status === 'accepted').length;
  const nominatedCount = PACE_ROLES.filter((role) => getTrusteeForRole(role.key)).length;

  useEffect(() => {
    fetchTrustees();
  }, [user?.did]);

  useEffect(() => {
    saveReplaceState(replaceMode, replaceEmail);
  }, [replaceMode, replaceEmail]);

  async function fetchTrustees() {
    if (!user?.did) return;
    setLoading(true);
    try {
      const response = await api.get(`/pace/trustees/${user.did}`);
      setTrustees(response.data.trustees || response.data || []);
      setVssCeremony(response.data.vss_ceremony || null);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load P.A.C.E. contacts.');
    } finally {
      setLoading(false);
    }
  }

  function getTrusteeForRole(roleKey) {
    const roleTrustees = normalizedTrustees.filter((trustee) => trustee.role === roleKey);
    return roleTrustees.find((trustee) => trustee.status === 'accepted') ||
      roleTrustees.find((trustee) => trustee.status === 'pending') ||
      roleTrustees.find((trustee) => trustee.status === 'declined') ||
      roleTrustees[0];
  }

  function openNomination(roleKey) {
    setActiveRole(roleKey);
    setInviteDraft({ email: '', phone: '', channels: defaultChannels() });
    setError('');
    setNotice('');
  }

  function updateDraft(patch) {
    setInviteDraft((prev) => ({
      ...prev,
      ...patch,
      channels: {
        ...prev.channels,
        ...(patch.channels || {}),
      },
    }));
  }

  function validateDraft() {
    const email = inviteDraft.email.trim().toLowerCase();
    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      return 'Enter a valid email address.';
    }
    if (user?.email && email === user.email.toLowerCase()) {
      return 'You cannot nominate yourself as a P.A.C.E. contact.';
    }
    if (selectedChannels(inviteDraft.channels).length === 0) {
      return 'Choose at least one invitation channel.';
    }
    if (inviteDraft.channels.sms && !inviteDraft.phone.trim()) {
      return 'SMS invitations require a phone number.';
    }
    const duplicate = normalizedTrustees.find((trustee) => trustee.email?.toLowerCase() === email && trustee.status !== 'replaced');
    if (duplicate) {
      return `${email} is already nominated as ${duplicate.role}. The same person cannot fill multiple P.A.C.E. roles.`;
    }
    return null;
  }

  async function handleNominate(event) {
    event.preventDefault();
    const validationError = validateDraft();
    if (validationError) {
      setError(validationError);
      return;
    }

    setSending(true);
    setError('');
    setNotice('');
    try {
      const role = PACE_ROLES.find((item) => item.key === activeRole);
      const response = await api.post('/pace/trustees', {
        subscriber_id: user.id,
        trustees: [{
          role: activeRole,
          email: inviteDraft.email.trim().toLowerCase(),
          phone: inviteDraft.phone.trim() || null,
          delivery_channels: selectedChannels(inviteDraft.channels),
        }],
      });
      const created = response.data?.[0];
      setNotice(`${role.name} invitation created. ${deliverySummary(created || {})}`);
      setActiveRole(null);
      await fetchTrustees();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to create invitation.');
    } finally {
      setSending(false);
    }
  }

  async function handleResend(trustee, channels) {
    setSending(true);
    setError('');
    setNotice('');
    try {
      const response = await api.post(`/pace/trustees/${trustee.id}/send`, {
        subscriber_did: user.did,
        delivery_channels: channels,
        phone: trustee.invite_phone || null,
      });
      setNotice(`Invitation updated for ${trustee.email}. ${deliverySummary(response.data)}`);
      await fetchTrustees();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to update invitation.');
    } finally {
      setSending(false);
    }
  }

  async function copyInvitationLink(trustee) {
    if (!trustee.invitation_url) {
      setError('No invitation link is available for this contact yet.');
      return;
    }
    try {
      await navigator.clipboard.writeText(trustee.invitation_url);
      setCopyStatus(`Copied link for ${trustee.email}`);
      setTimeout(() => setCopyStatus(''), 3000);
    } catch {
      setError('Clipboard permission was denied. Open the invitation link from the contact row and copy it manually.');
    }
  }

  async function handleReplace(trusteeId) {
    if (!replaceEmail.trim()) return;
    setReplaceSubmitting(true);
    setError('');
    setNotice('');
    try {
      const response = await api.post(`/pace/trustees/${trusteeId}/replace`, {
        new_email: replaceEmail.trim().toLowerCase(),
        subscriber_did: user.did,
      });
      setNotice(`Replacement workflow created. Workflow ${response.data.workflow_id} needs trustee approvals.`);
      setReplaceMode(null);
      setReplaceEmail('');
      await fetchTrustees();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to create replacement workflow.');
    } finally {
      setReplaceSubmitting(false);
    }
  }

  return (
    <div className="min-h-screen bg-slate-50">
      <Navbar />
      <main id="main-content" tabIndex={-1} className="mx-auto max-w-6xl px-4 py-8 sm:px-6 lg:px-8">
        <header className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold text-slate-950">P.A.C.E. Safety Circle</h1>
            <p className="mt-1 text-sm text-slate-600">
              Invite your four by Email, SMS, or Copy link. A complete circle is Primary, Alternate, Contingent, and Emergency.
            </p>
          </div>
          <div className="rounded-md border border-slate-200 bg-white px-4 py-3 text-sm shadow-sm">
            <span className="font-semibold text-slate-950">{nominatedCount}/4 nominated</span>
            <span className="mx-2 text-slate-300">|</span>
            <span className="font-semibold text-teal-700">{acceptedCount}/4 accepted</span>
          </div>
        </header>

        {(notice || copyStatus) && (
          <div className="mb-4 rounded-md border border-teal-200 bg-teal-50 p-3 text-sm text-teal-800" data-testid="pace-notice">
            {copyStatus || notice}
          </div>
        )}
        {error && (
          <div className="mb-4 rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700" data-testid="pace-error" role="alert">
            {error}
          </div>
        )}

        {loading ? (
          <div className="rounded-lg border border-slate-200 bg-white p-8 text-center text-sm text-slate-600">Loading P.A.C.E. contacts...</div>
        ) : (
          <div className="grid gap-4 lg:grid-cols-2" data-testid="pace-slots">
            {PACE_ROLES.map((role) => {
              const trustee = getTrusteeForRole(role.key);
              return (
                <section key={role.key} className="rounded-lg border border-slate-200 bg-white p-4 shadow-sm" data-testid={`pace-slot-${role.key}`}>
                  <div className="flex items-start gap-3">
                    <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-md bg-slate-900 text-base font-bold text-white">
                      {role.letter}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <h2 className="font-semibold text-slate-950">{role.name}</h2>
                        {trustee && (
                          <span className="rounded-full bg-slate-100 px-2 py-0.5 text-xs font-semibold text-slate-700" data-testid={`trustee-status-${role.key}`}>
                            {statusLabel(trustee.status)}
                          </span>
                        )}
                      </div>
                      <p className="mt-1 text-sm leading-6 text-slate-600">{role.description}</p>

                      {trustee ? (
                        <div className="mt-4 space-y-3">
                          <div className="rounded-md border border-slate-200 bg-slate-50 p-3">
                            <p className="break-all text-sm font-semibold text-slate-900" data-testid={`trustee-email-${role.key}`}>{trustee.email}</p>
                            {trustee.invite_phone && <p className="mt-1 text-xs text-slate-600">{trustee.invite_phone}</p>}
                            <p className="mt-2 text-xs text-slate-600">{deliverySummary(trustee)}</p>
                            {trustee.accepted_at && <p className="mt-1 text-xs text-slate-500">Accepted {new Date(trustee.accepted_at).toLocaleDateString()}</p>}
                          </div>

                          {trustee.status !== 'accepted' && (
                            <div className="flex flex-wrap gap-2">
                              <button
                                type="button"
                                onClick={() => handleResend(trustee, ['email', 'link'])}
                                disabled={sending}
                                className="rounded-md border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-100 disabled:opacity-60"
                              >
                                Email
                              </button>
                              <button
                                type="button"
                                onClick={() => handleResend(trustee, ['sms', 'link'])}
                                disabled={sending}
                                className="rounded-md border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-100 disabled:opacity-60"
                              >
                                SMS
                              </button>
                              <button
                                type="button"
                                onClick={() => copyInvitationLink(trustee)}
                                className="rounded-md bg-teal-700 px-3 py-2 text-sm font-semibold text-white hover:bg-teal-800"
                              >
                                Copy link
                              </button>
                            </div>
                          )}

                          {trustee.status === 'accepted' && (
                            replaceMode === trustee.id ? (
                              <div className="rounded-md border border-orange-200 bg-orange-50 p-3">
                                <label className="block text-sm font-medium text-slate-700">
                                  Replacement email
                                  <input
                                    type="email"
                                    value={replaceEmail}
                                    onChange={(event) => setReplaceEmail(event.target.value)}
                                    className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-orange-600 focus:outline-none focus:ring-2 focus:ring-orange-100"
                                    placeholder="new.person@example.com"
                                  />
                                </label>
                                <div className="mt-2 flex flex-wrap gap-2">
                                  <button
                                    type="button"
                                    onClick={() => handleReplace(trustee.id)}
                                    disabled={replaceSubmitting || !replaceEmail.trim()}
                                    className="rounded-md bg-orange-600 px-3 py-2 text-sm font-semibold text-white hover:bg-orange-700 disabled:opacity-60"
                                  >
                                    Confirm replacement
                                  </button>
                                  <button
                                    type="button"
                                    onClick={() => { setReplaceMode(null); setReplaceEmail(''); }}
                                    className="rounded-md border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-white"
                                  >
                                    Cancel
                                  </button>
                                </div>
                              </div>
                            ) : (
                              <button
                                type="button"
                                onClick={() => { setReplaceMode(trustee.id); setReplaceEmail(''); }}
                                className="rounded-md border border-orange-300 px-3 py-2 text-sm font-semibold text-orange-700 hover:bg-orange-50"
                              >
                                Replace contact
                              </button>
                            )
                          )}
                        </div>
                      ) : (
                        <button
                          type="button"
                          onClick={() => openNomination(role.key)}
                          className="mt-4 rounded-md bg-teal-700 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-800"
                          data-testid={`nominate-btn-${role.key}`}
                        >
                          Invite {role.name}
                        </button>
                      )}
                    </div>
                  </div>
                </section>
              );
            })}
          </div>
        )}

        {vssCeremony && (
          <section className="mt-6 rounded-lg border border-slate-200 bg-white p-4 shadow-sm" data-testid="vss-shard-health">
            <h2 className="font-semibold text-slate-950">Shard readiness</h2>
            <p className="mt-1 text-sm text-slate-600">
              Ceremony {vssCeremony.status}. Threshold {vssCeremony.threshold}-of-{vssCeremony.total_shares}.
            </p>
          </section>
        )}

        {activeRole && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/50 p-4">
            <div className="w-full max-w-lg rounded-lg bg-white p-5 shadow-xl">
              <h2 className="text-lg font-semibold text-slate-950">
                Invite {PACE_ROLES.find((role) => role.key === activeRole)?.name}
              </h2>
              <p className="mt-1 text-sm text-slate-600">
                The invite says this is not a marketing invite and that the person can accept, decline, or ask you to choose someone else.
              </p>
              <form onSubmit={handleNominate} className="mt-4 space-y-4">
                <label className="block text-sm font-medium text-slate-700">
                  Email
                  <input
                    type="email"
                    value={inviteDraft.email}
                    onChange={(event) => updateDraft({ email: event.target.value })}
                    className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100"
                    placeholder="person@example.com"
                    autoFocus
                    data-testid="trustee-email-input"
                  />
                </label>
                <label className="block text-sm font-medium text-slate-700">
                  SMS phone
                  <input
                    type="tel"
                    value={inviteDraft.phone}
                    onChange={(event) => updateDraft({ phone: event.target.value })}
                    className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100"
                    placeholder="+1 555 123 4567"
                  />
                </label>
                <fieldset>
                  <legend className="text-sm font-medium text-slate-700">Invitation channels</legend>
                  <div className="mt-2 flex flex-wrap gap-2">
                    {CHANNELS.map((channel) => (
                      <label key={channel.key} className="inline-flex items-center gap-2 rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700">
                        <input
                          type="checkbox"
                          checked={inviteDraft.channels[channel.key]}
                          onChange={(event) => updateDraft({ channels: { [channel.key]: event.target.checked } })}
                          className="h-4 w-4 rounded border-slate-300 text-teal-700 focus:ring-teal-700"
                        />
                        {channel.label}
                      </label>
                    ))}
                  </div>
                </fieldset>
                <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
                  <button
                    type="button"
                    onClick={() => setActiveRole(null)}
                    className="rounded-md border border-slate-300 px-4 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-100"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={sending}
                    className="rounded-md bg-teal-700 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-800 disabled:opacity-60"
                  >
                    {sending ? 'Creating...' : 'Create invitation'}
                  </button>
                </div>
              </form>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
