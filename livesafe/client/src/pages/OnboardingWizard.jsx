import React, { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import { PACE_ROLES } from '../lib/paceRoles';

const WIZARD_STORAGE_KEY = 'onboarding_wizard_progress';
const WIZARD_STEP_KEY = 'onboarding_wizard_step';
const BLOOD_TYPES = ['', 'A+', 'A-', 'B+', 'B-', 'AB+', 'AB-', 'O+', 'O-', 'Unknown'];

const STEPS = [
  { id: 1, label: 'Card facts' },
  { id: 2, label: 'Medical notes' },
  { id: 3, label: 'Safety Circle' },
  { id: 4, label: 'Review' },
];

function defaultPaceInvites() {
  return PACE_ROLES.reduce((acc, role) => {
    acc[role.key] = {
      email: '',
      phone: '',
      channels: { email: true, sms: false, link: true },
    };
    return acc;
  }, {});
}

function loadWizardProgress() {
  try {
    const raw = sessionStorage.getItem(WIZARD_STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function loadWizardStep() {
  try {
    const raw = sessionStorage.getItem(WIZARD_STEP_KEY);
    return raw ? Number(raw) || 1 : 1;
  } catch {
    return 1;
  }
}

function saveWizardProgress(data) {
  try {
    sessionStorage.setItem(WIZARD_STORAGE_KEY, JSON.stringify(data));
  } catch {
    /* session persistence is best-effort */
  }
}

function saveWizardStep(step) {
  try {
    sessionStorage.setItem(WIZARD_STEP_KEY, String(step));
  } catch {
    /* session persistence is best-effort */
  }
}

function clearWizardStorage() {
  try {
    sessionStorage.removeItem(WIZARD_STORAGE_KEY);
    sessionStorage.removeItem(WIZARD_STEP_KEY);
  } catch {
    /* session persistence is best-effort */
  }
}

function selectedChannels(invite) {
  return Object.entries(invite.channels)
    .filter(([, enabled]) => enabled)
    .map(([channel]) => channel);
}

export default function OnboardingWizard() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const saved = useMemo(loadWizardProgress, []);

  const [currentStep, setCurrentStepState] = useState(loadWizardStep);
  const [firstName, setFirstName] = useState(saved.firstName || user?.first_name || '');
  const [lastName, setLastName] = useState(saved.lastName || user?.last_name || '');
  const [dateOfBirth, setDateOfBirth] = useState(saved.dateOfBirth || '');
  const [bloodType, setBloodType] = useState(saved.bloodType || '');
  const [allergies, setAllergies] = useState(saved.allergies || '');
  const [medications, setMedications] = useState(saved.medications || '');
  const [conditions, setConditions] = useState(saved.conditions || '');
  const [dnrStatus, setDnrStatus] = useState(saved.dnrStatus || 'not_specified');
  const [emergencyContacts, setEmergencyContacts] = useState(saved.emergencyContacts || []);
  const [contactDraft, setContactDraft] = useState({ name: '', phone: '', relationship: '' });
  const [paceInvites, setPaceInvites] = useState({
    ...defaultPaceInvites(),
    ...(saved.paceInvites || {}),
  });
  const [errors, setErrors] = useState({});
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState('');
  const [inviteResults, setInviteResults] = useState([]);
  const [completed, setCompleted] = useState(false);

  const invitedCount = Object.values(paceInvites).filter((invite) => invite.email.trim()).length;
  const progressPercent = Math.round(((currentStep - 1) / STEPS.length) * 100);

  useEffect(() => {
    saveWizardProgress({
      firstName,
      lastName,
      dateOfBirth,
      bloodType,
      allergies,
      medications,
      conditions,
      dnrStatus,
      emergencyContacts,
      paceInvites,
    });
  }, [
    firstName,
    lastName,
    dateOfBirth,
    bloodType,
    allergies,
    medications,
    conditions,
    dnrStatus,
    emergencyContacts,
    paceInvites,
  ]);

  function setCurrentStep(step) {
    const bounded = Math.max(1, Math.min(STEPS.length, step));
    setCurrentStepState(bounded);
    saveWizardStep(bounded);
  }

  function validateStep1() {
    const nextErrors = {};
    if (!firstName.trim()) nextErrors.firstName = 'First name is required';
    if (!lastName.trim()) nextErrors.lastName = 'Last name is required';
    if (dateOfBirth && new Date(dateOfBirth) >= new Date()) {
      nextErrors.dateOfBirth = 'Date of birth must be in the past';
    }
    setErrors(nextErrors);
    return Object.keys(nextErrors).length === 0;
  }

  function validateStep3() {
    const nextErrors = {};
    for (const role of PACE_ROLES) {
      const invite = paceInvites[role.key];
      if (!invite.email.trim()) continue;
      if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(invite.email.trim())) {
        nextErrors[`${role.key}.email`] = 'Enter a valid email address';
      }
      if (invite.channels.sms && !invite.phone.trim()) {
        nextErrors[`${role.key}.phone`] = 'SMS requires a phone number';
      }
      if (selectedChannels(invite).length === 0) {
        nextErrors[`${role.key}.channels`] = 'Choose at least one invitation channel';
      }
    }
    setErrors(nextErrors);
    return Object.keys(nextErrors).length === 0;
  }

  function nextStep() {
    if (currentStep === 1 && !validateStep1()) return;
    if (currentStep === 3 && !validateStep3()) return;
    setErrors({});
    setCurrentStep(currentStep + 1);
  }

  function updatePaceInvite(roleKey, patch) {
    setPaceInvites((prev) => ({
      ...prev,
      [roleKey]: {
        ...prev[roleKey],
        ...patch,
        channels: {
          ...prev[roleKey].channels,
          ...(patch.channels || {}),
        },
      },
    }));
  }

  function addEmergencyContact() {
    const name = contactDraft.name.trim();
    const phone = contactDraft.phone.trim();
    if (!name || !phone) {
      setErrors((prev) => ({ ...prev, contact: 'Contact name and phone are required' }));
      return;
    }
    setEmergencyContacts((prev) => [
      ...prev,
      { name, phone, relationship: contactDraft.relationship.trim() },
    ]);
    setContactDraft({ name: '', phone: '', relationship: '' });
    setErrors((prev) => ({ ...prev, contact: '' }));
  }

  async function writeCommaList(value, endpoint, payloadKey, extras = {}) {
    const items = value.split(',').map((item) => item.trim()).filter(Boolean);
    for (const item of items) {
      try {
        await api.post(endpoint, { [payloadKey]: item, ...extras });
      } catch {
        /* duplicate profile facts should not block onboarding completion */
      }
    }
  }

  async function submitInvitations() {
    const trustees = PACE_ROLES
      .map((role) => ({ role, invite: paceInvites[role.key] }))
      .filter(({ invite }) => invite.email.trim())
      .map(({ role, invite }) => ({
        role: role.key,
        email: invite.email.trim().toLowerCase(),
        phone: invite.phone.trim() || null,
        delivery_channels: selectedChannels(invite),
      }));

    if (trustees.length === 0) {
      return [];
    }

    const response = await api.post('/pace/trustees', {
      subscriber_id: user.id,
      trustees,
    });
    return response.data;
  }

  async function handleComplete() {
    const profileValid = validateStep1();
    const paceValid = validateStep3();
    if (!profileValid || !paceValid) {
      setCurrentStep(!profileValid ? 1 : 3);
      return;
    }

    setSubmitting(true);
    setSubmitError('');
    try {
      await api.put('/subscribers/profile', {
        first_name: firstName.trim(),
        last_name: lastName.trim(),
        date_of_birth: dateOfBirth || null,
        blood_type: bloodType || null,
        dnr_status: dnrStatus,
      });

      await writeCommaList(allergies, '/subscribers/profile/allergies', 'allergy', { severity: 'unknown' });
      await writeCommaList(medications, '/subscribers/profile/medications', 'medication', { dosage: '', frequency: '' });
      await writeCommaList(conditions, '/subscribers/profile/conditions', 'condition_name', { notes: '' });

      for (const contact of emergencyContacts) {
        try {
          await api.post('/subscribers/profile/emergency-contacts', {
            name: contact.name,
            phone: contact.phone,
            relationship: contact.relationship || null,
          });
        } catch {
          /* duplicate emergency contacts should not block onboarding completion */
        }
      }

      const results = await submitInvitations();
      setInviteResults(results);
      clearWizardStorage();
      setCompleted(true);
    } catch (err) {
      setSubmitError(err.response?.data?.error || err.message || 'Failed to complete onboarding.');
    } finally {
      setSubmitting(false);
    }
  }

  if (completed) {
    return (
      <div className="min-h-screen bg-slate-50 px-4 py-10">
        <div className="mx-auto max-w-xl rounded-lg border border-slate-200 bg-white p-6 shadow-sm" data-testid="onboarding-complete">
          <h1 className="text-2xl font-bold text-slate-950">Safety Circle setup saved</h1>
          <p className="mt-2 text-sm leading-6 text-slate-600">
            Your emergency-card facts were saved. Invitations were created for {inviteResults.length || invitedCount} P.A.C.E. contact{(inviteResults.length || invitedCount) === 1 ? '' : 's'} with copy links available immediately.
          </p>
          <div className="mt-5 grid gap-2 sm:grid-cols-2">
            <Link to="/pace" className="rounded-md bg-teal-700 px-4 py-3 text-center text-sm font-semibold text-white hover:bg-teal-800" data-testid="go-to-pace-link">
              Manage invitations
            </Link>
            <Link to="/dashboard" className="rounded-md border border-slate-300 px-4 py-3 text-center text-sm font-semibold text-slate-800 hover:bg-slate-100" data-testid="go-to-dashboard-link">
              Open dashboard
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-50 px-4 py-8">
      <div className="mx-auto max-w-4xl">
        <header className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <Link to="/" className="text-xl font-bold text-slate-950">
              LiveSafe<span className="text-teal-700">.ai</span>
            </Link>
            <h1 className="mt-3 text-3xl font-bold text-slate-950">Create your card. Invite your four. Protect your people.</h1>
            <p className="mt-2 text-sm text-slate-600">
              Complete the information responders may need, then invite your P.A.C.E. Safety Circle by Email, SMS, or Copy link.
            </p>
          </div>
          <span className="rounded-full bg-teal-50 px-3 py-1 text-sm font-semibold text-teal-800">
            {invitedCount}/4 invited
          </span>
        </header>

        <div className="mb-6" data-testid="wizard-progress-container">
          <div className="mb-2 flex items-center justify-between text-sm">
            <span className="font-medium text-slate-600" data-testid="wizard-step-label">Step {currentStep} of {STEPS.length}</span>
            <span className="font-semibold text-teal-700" data-testid="wizard-progress-percent">{progressPercent}% complete</span>
          </div>
          <div className="h-2 rounded-full bg-slate-200" role="progressbar" aria-valuenow={progressPercent} aria-valuemin={0} aria-valuemax={100} data-testid="wizard-progress-bar">
            <div className="h-2 rounded-full bg-teal-700 transition-all" style={{ width: `${progressPercent}%` }} />
          </div>
          <div className="mt-3 grid grid-cols-4 gap-2" data-testid="wizard-steps">
            {STEPS.map((step) => (
              <button
                key={step.id}
                type="button"
                onClick={() => setCurrentStep(step.id)}
                className={`rounded-md border px-2 py-2 text-xs font-semibold ${currentStep === step.id ? 'border-teal-700 bg-teal-50 text-teal-800' : 'border-slate-200 bg-white text-slate-600'}`}
                data-testid={`wizard-step-${step.id}`}
                data-active={currentStep === step.id}
                data-completed={currentStep > step.id}
              >
                {step.label}
              </button>
            ))}
          </div>
        </div>

        <section className="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
          {currentStep === 1 && (
            <div data-testid="wizard-step-1-panel">
              <h2 className="text-xl font-semibold text-slate-950">Card facts</h2>
              <p className="mt-1 text-sm text-slate-600">These fields identify you and shape emergency-card readiness.</p>
              <div className="mt-5 grid gap-4 sm:grid-cols-2">
                <label className="block text-sm font-medium text-slate-700">
                  First name
                  <input value={firstName} onChange={(e) => setFirstName(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" />
                  {errors.firstName && <span className="mt-1 block text-xs text-red-700">{errors.firstName}</span>}
                </label>
                <label className="block text-sm font-medium text-slate-700">
                  Last name
                  <input value={lastName} onChange={(e) => setLastName(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" />
                  {errors.lastName && <span className="mt-1 block text-xs text-red-700">{errors.lastName}</span>}
                </label>
                <label className="block text-sm font-medium text-slate-700">
                  Date of birth
                  <input type="date" value={dateOfBirth} onChange={(e) => setDateOfBirth(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" />
                  {errors.dateOfBirth && <span className="mt-1 block text-xs text-red-700">{errors.dateOfBirth}</span>}
                </label>
                <label className="block text-sm font-medium text-slate-700">
                  Blood type
                  <select value={bloodType} onChange={(e) => setBloodType(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100">
                    {BLOOD_TYPES.map((type) => <option key={type || 'blank'} value={type}>{type || 'Select'}</option>)}
                  </select>
                </label>
              </div>
            </div>
          )}

          {currentStep === 2 && (
            <div data-testid="wizard-step-2-panel">
              <h2 className="text-xl font-semibold text-slate-950">Medical notes</h2>
              <p className="mt-1 text-sm text-slate-600">Use comma-separated entries where useful. Keep this to emergency-relevant facts.</p>
              <div className="mt-5 space-y-4">
                <label className="block text-sm font-medium text-slate-700">
                  Allergies
                  <textarea value={allergies} onChange={(e) => setAllergies(e.target.value)} rows={3} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" placeholder="Penicillin, peanuts" />
                </label>
                <label className="block text-sm font-medium text-slate-700">
                  Medications
                  <textarea value={medications} onChange={(e) => setMedications(e.target.value)} rows={3} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" placeholder="Insulin, EpiPen" />
                </label>
                <label className="block text-sm font-medium text-slate-700">
                  Conditions
                  <textarea value={conditions} onChange={(e) => setConditions(e.target.value)} rows={3} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" placeholder="Asthma, diabetes" />
                </label>
                <label className="block text-sm font-medium text-slate-700">
                  DNR status
                  <select value={dnrStatus} onChange={(e) => setDnrStatus(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100">
                    <option value="not_specified">Not specified</option>
                    <option value="none">No DNR on file</option>
                    <option value="dnr_on_file">DNR on file</option>
                  </select>
                </label>
              </div>
            </div>
          )}

          {currentStep === 3 && (
            <div data-testid="wizard-step-3-panel">
              <h2 className="text-xl font-semibold text-slate-950">Invite your P.A.C.E. Safety Circle</h2>
              <p className="mt-1 text-sm text-slate-600">
                Add one person for each role. Every invitation includes autonomy language: this is not a marketing invite, and the person can accept, decline, or ask you to choose someone else.
              </p>

              <div className="mt-5 divide-y divide-slate-200 rounded-md border border-slate-200">
                {PACE_ROLES.map((role) => {
                  const invite = paceInvites[role.key];
                  return (
                    <div key={role.key} className="grid gap-4 p-4 lg:grid-cols-[180px_1fr]">
                      <div>
                        <div className="flex items-center gap-3">
                          <span className="flex h-9 w-9 items-center justify-center rounded-md bg-slate-900 text-sm font-bold text-white">{role.letter}</span>
                          <div>
                            <p className="font-semibold text-slate-950">{role.name}</p>
                            <p className="text-xs text-slate-600">{role.description}</p>
                          </div>
                        </div>
                      </div>
                      <div className="grid gap-3 sm:grid-cols-2">
                        <label className="block text-sm font-medium text-slate-700">
                          Email
                          <input type="email" value={invite.email} onChange={(e) => updatePaceInvite(role.key, { email: e.target.value })} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" placeholder="person@example.com" />
                          {errors[`${role.key}.email`] && <span className="mt-1 block text-xs text-red-700">{errors[`${role.key}.email`]}</span>}
                        </label>
                        <label className="block text-sm font-medium text-slate-700">
                          SMS phone
                          <input type="tel" value={invite.phone} onChange={(e) => updatePaceInvite(role.key, { phone: e.target.value })} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-base focus:border-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-100" placeholder="+1 555 123 4567" />
                          {errors[`${role.key}.phone`] && <span className="mt-1 block text-xs text-red-700">{errors[`${role.key}.phone`]}</span>}
                        </label>
                        <div className="sm:col-span-2">
                          <p className="text-sm font-medium text-slate-700">Invitation channels</p>
                          <div className="mt-2 flex flex-wrap gap-2">
                            {[
                              ['email', 'Email'],
                              ['sms', 'SMS'],
                              ['link', 'Copy link'],
                            ].map(([key, label]) => (
                              <label key={key} className="inline-flex items-center gap-2 rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700">
                                <input
                                  type="checkbox"
                                  checked={invite.channels[key]}
                                  onChange={(e) => updatePaceInvite(role.key, { channels: { [key]: e.target.checked } })}
                                  className="h-4 w-4 rounded border-slate-300 text-teal-700 focus:ring-teal-700"
                                />
                                {label}
                              </label>
                            ))}
                          </div>
                          {errors[`${role.key}.channels`] && <span className="mt-1 block text-xs text-red-700">{errors[`${role.key}.channels`]}</span>}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>

              <div className="mt-6">
                <h3 className="text-sm font-semibold text-slate-950">Emergency contacts on card</h3>
                <div className="mt-2 grid gap-2 sm:grid-cols-4">
                  <input value={contactDraft.name} onChange={(e) => setContactDraft((prev) => ({ ...prev, name: e.target.value }))} className="rounded-md border border-slate-300 px-3 py-2 text-base" placeholder="Name" />
                  <input value={contactDraft.phone} onChange={(e) => setContactDraft((prev) => ({ ...prev, phone: e.target.value }))} className="rounded-md border border-slate-300 px-3 py-2 text-base" placeholder="Phone" />
                  <input value={contactDraft.relationship} onChange={(e) => setContactDraft((prev) => ({ ...prev, relationship: e.target.value }))} className="rounded-md border border-slate-300 px-3 py-2 text-base" placeholder="Relationship" />
                  <button type="button" onClick={addEmergencyContact} className="rounded-md border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-800 hover:bg-slate-100">Add</button>
                </div>
                {errors.contact && <p className="mt-1 text-xs text-red-700">{errors.contact}</p>}
                {emergencyContacts.length > 0 && (
                  <ul className="mt-3 divide-y divide-slate-200 rounded-md border border-slate-200 text-sm">
                    {emergencyContacts.map((contact, index) => (
                      <li key={`${contact.phone}-${index}`} className="flex items-center justify-between gap-3 px-3 py-2">
                        <span>{contact.name} - {contact.phone}{contact.relationship ? ` - ${contact.relationship}` : ''}</span>
                        <button type="button" onClick={() => setEmergencyContacts((prev) => prev.filter((_, i) => i !== index))} className="text-sm font-semibold text-red-700">Remove</button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          )}

          {currentStep === 4 && (
            <div data-testid="wizard-step-4-panel">
              <h2 className="text-xl font-semibold text-slate-950">Review and submit</h2>
              <p className="mt-1 text-sm text-slate-600">LiveSafe will save your card facts and create bearer invitation links for each P.A.C.E. role with an email address.</p>
              <dl className="mt-5 grid gap-3 text-sm sm:grid-cols-2">
                <div className="rounded-md border border-slate-200 p-3">
                  <dt className="font-semibold text-slate-900">Profile</dt>
                  <dd className="mt-1 text-slate-600">{firstName || 'First'} {lastName || 'Last'}{bloodType ? ` - ${bloodType}` : ''}</dd>
                </div>
                <div className="rounded-md border border-slate-200 p-3">
                  <dt className="font-semibold text-slate-900">Safety Circle</dt>
                  <dd className="mt-1 text-slate-600">{invitedCount} of 4 invitation slots prepared</dd>
                </div>
                <div className="rounded-md border border-slate-200 p-3">
                  <dt className="font-semibold text-slate-900">Channels</dt>
                  <dd className="mt-1 text-slate-600">Email, SMS, and Copy link delivery status will be shown on the P.A.C.E. page.</dd>
                </div>
                <div className="rounded-md border border-slate-200 p-3">
                  <dt className="font-semibold text-slate-900">Autonomy</dt>
                  <dd className="mt-1 text-slate-600">Invitees can accept, decline, or ask you to choose someone else.</dd>
                </div>
              </dl>
              {submitError && <div className="mt-4 rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">{submitError}</div>}
            </div>
          )}

          <div className="mt-6 flex flex-col-reverse gap-2 sm:flex-row sm:justify-between">
            <button
              type="button"
              onClick={() => setCurrentStep(currentStep - 1)}
              disabled={currentStep === 1}
              className="rounded-md border border-slate-300 px-4 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-50"
              data-testid={`wizard-step${currentStep}-back`}
            >
              Back
            </button>
            {currentStep < STEPS.length ? (
              <button
                type="button"
                onClick={nextStep}
                className="rounded-md bg-teal-700 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-800"
                data-testid={`wizard-step${currentStep}-next`}
              >
                Continue
              </button>
            ) : (
              <button
                type="button"
                onClick={handleComplete}
                disabled={submitting}
                className="rounded-md bg-teal-700 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-800 disabled:cursor-not-allowed disabled:opacity-60"
                data-testid="wizard-complete"
              >
                {submitting ? 'Saving...' : 'Save and send invitations'}
              </button>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
