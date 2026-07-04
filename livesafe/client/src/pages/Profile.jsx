import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Link, useBlocker } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

const DNR_OPTIONS = ['not_specified', 'full_code', 'dnr', 'dnr_comfort_only', 'limited_intervention'];

export default function Profile() {
  const { user } = useAuth();
  const [loading, setLoading] = useState(true);
  const [bloodTypes, setBloodTypes] = useState(['']);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');

  // Ref-based guard to prevent concurrent profile saves (Feature #266)
  // useRef is synchronously updated, unlike useState which is batched by React
  const isSavingRef = useRef(false);

  // Core profile fields
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [dateOfBirth, setDateOfBirth] = useState('');
  const [bloodType, setBloodType] = useState('');
  const [dnrStatus, setDnrStatus] = useState('not_specified');
  const [organDonor, setOrganDonor] = useState(false);

  // Lists
  const [allergies, setAllergies] = useState([]);
  const [medications, setMedications] = useState([]);
  const [conditions, setConditions] = useState([]);
  const [emergencyContacts, setEmergencyContacts] = useState([]);

  // New item inputs
  const [newAllergy, setNewAllergy] = useState('');
  const [newAllergySeverity, setNewAllergySeverity] = useState('');
  const [newMedication, setNewMedication] = useState('');
  const [newMedDosage, setNewMedDosage] = useState('');
  const [newMedFrequency, setNewMedFrequency] = useState('');
  const [newCondition, setNewCondition] = useState('');
  const [newCondDiagnosed, setNewCondDiagnosed] = useState('');
  const [newCondNotes, setNewCondNotes] = useState('');
  const [newContactName, setNewContactName] = useState('');
  const [newContactPhone, setNewContactPhone] = useState('');
  const [newContactRelationship, setNewContactRelationship] = useState('');

  // Phone verification state
  const [phone, setPhone] = useState('');
  const [phoneVerified, setPhoneVerified] = useState(false);
  const [phoneInput, setPhoneInput] = useState('');
  const [phoneVerificationCode, setPhoneVerificationCode] = useState('');
  const [phonePending, setPhonePending] = useState(false); // code requested, awaiting confirm
  const [phoneDevCode, setPhoneDevCode] = useState('');
  const [phoneMessage, setPhoneMessage] = useState('');
  const [phoneSaving, setPhoneSaving] = useState(false);

  // Edit emergency contact state
  const [editingContactId, setEditingContactId] = useState(null);
  const [editContactName, setEditContactName] = useState('');
  const [editContactPhone, setEditContactPhone] = useState('');
  const [editContactRelationship, setEditContactRelationship] = useState('');

  // Phone validation error state
  const [contactPhoneError, setContactPhoneError] = useState('');
  const [editContactPhoneError, setEditContactPhoneError] = useState('');

  // Track unsaved changes
  const [isDirty, setIsDirty] = useState(false);
  const [savedProfile, setSavedProfile] = useState(null);

  const loadProfile = useCallback(async () => {
    try {
      // Fetch blood types from backend
      const btRes = await api.get('/subscribers/blood-types');
      const fetchedTypes = btRes.data.blood_types || [];
      setBloodTypes(['', ...fetchedTypes]);

      const res = await api.get('/subscribers/profile');
      const p = res.data;
      const fn = p.first_name || '';
      const ln = p.last_name || '';
      const dob = p.date_of_birth ? p.date_of_birth.split('T')[0] : '';
      const bt = p.blood_type || '';
      const dnr = p.dnr_status || 'not_specified';
      const od = p.organ_donor || false;
      setFirstName(fn);
      setLastName(ln);
      setDateOfBirth(dob);
      setBloodType(bt);
      setDnrStatus(dnr);
      setOrganDonor(od);
      setAllergies(p.allergies || []);
      setMedications(p.medications || []);
      setConditions(p.conditions || []);
      setEmergencyContacts(p.emergency_contacts || []);
      // Phone verification
      setPhone(p.phone || '');
      setPhoneInput(p.phone || '');
      setPhoneVerified(p.phone_verified || false);
      setPhonePending(false);
      setPhoneDevCode('');
      setPhoneMessage('');
      // Store saved profile snapshot for dirty detection
      setSavedProfile({ first_name: fn, last_name: ln, date_of_birth: dob, blood_type: bt, dnr_status: dnr, organ_donor: od });
      setIsDirty(false);
    } catch (err) {
      console.error('Failed to load profile:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadProfile(); }, [loadProfile]);

  // Detect if any basic profile field has changed from saved values
  useEffect(() => {
    if (!savedProfile) return;
    const changed =
      firstName !== savedProfile.first_name ||
      lastName !== savedProfile.last_name ||
      dateOfBirth !== savedProfile.date_of_birth ||
      bloodType !== savedProfile.blood_type ||
      dnrStatus !== savedProfile.dnr_status ||
      organDonor !== savedProfile.organ_donor;
    setIsDirty(changed);
  }, [firstName, lastName, dateOfBirth, bloodType, dnrStatus, organDonor, savedProfile]);

  // Block navigation when there are unsaved changes
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      isDirty && currentLocation.pathname !== nextLocation.pathname
  );

  // Feature #357: Focus trap ref for unsaved changes dialog
  const unsavedDialogRef = useRef(null);

  // Feature #357: Auto-focus and focus trap for unsaved changes dialog
  useEffect(() => {
    if (blocker.state !== 'blocked') return;
    if (unsavedDialogRef.current) {
      const focusable = unsavedDialogRef.current.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (focusable.length > 0) focusable[0].focus();
    }
  }, [blocker.state]);

  const validateDateOfBirth = (dob) => {
    if (!dob) return null; // DOB is optional
    const dobDate = new Date(dob);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    // Reject future dates
    if (dobDate >= today) {
      return 'Date of birth cannot be in the future';
    }
    // Reject dates more than 150 years ago
    const minDate = new Date();
    minDate.setFullYear(minDate.getFullYear() - 150);
    if (dobDate < minDate) {
      return 'Date of birth cannot be more than 150 years ago';
    }
    return null;
  };

  const handleSaveProfile = async (e) => {
    e.preventDefault();

    // Prevent concurrent/double saves — synchronous ref check (Feature #266)
    if (isSavingRef.current) {
      return;
    }
    isSavingRef.current = true;

    setSaving(true);
    setMessage('');

    // Validate required fields
    if (!firstName.trim()) {
      setMessage('First name is required');
      setSaving(false);
      isSavingRef.current = false;
      return;
    }
    if (!lastName.trim()) {
      setMessage('Last name is required');
      setSaving(false);
      isSavingRef.current = false;
      return;
    }

    // Validate blood type is selected
    if (!bloodType) {
      setMessage('Please select a blood type');
      setSaving(false);
      isSavingRef.current = false;
      return;
    }

    // Validate date of birth
    const dobError = validateDateOfBirth(dateOfBirth);
    if (dobError) {
      setMessage(dobError);
      setSaving(false);
      isSavingRef.current = false;
      return;
    }

    try {
      await api.put('/subscribers/profile', {
        first_name: firstName,
        last_name: lastName,
        date_of_birth: dateOfBirth || null,
        blood_type: bloodType || null,
        dnr_status: dnrStatus,
        organ_donor: organDonor,
      });
      // Update saved profile snapshot and clear dirty flag
      setSavedProfile({ first_name: firstName, last_name: lastName, date_of_birth: dateOfBirth, blood_type: bloodType, dnr_status: dnrStatus, organ_donor: organDonor });
      setIsDirty(false);
      setMessage('Profile saved successfully!');
      setTimeout(() => setMessage(''), 3000);
    } catch (err) {
      setMessage('Failed to save profile');
    } finally {
      setSaving(false);
      isSavingRef.current = false;
    }
  };

  const addAllergy = async () => {
    if (!newAllergy.trim()) return;
    try {
      const res = await api.post('/subscribers/profile/allergies', { allergy: newAllergy.trim(), severity: newAllergySeverity || null });
      setAllergies([...allergies, res.data]);
      setNewAllergy('');
      setNewAllergySeverity('');
    } catch (err) { console.error('Failed to add allergy:', err); }
  };

  const removeAllergy = async (id) => {
    try {
      await api.delete(`/subscribers/profile/allergies/${id}`);
      setAllergies(allergies.filter(a => a.id !== id));
    } catch (err) { console.error('Failed to remove allergy:', err); }
  };

  const addMedication = async () => {
    if (!newMedication.trim()) return;
    try {
      const res = await api.post('/subscribers/profile/medications', { medication: newMedication.trim(), dosage: newMedDosage || null, frequency: newMedFrequency || null });
      setMedications([...medications, res.data]);
      setNewMedication('');
      setNewMedDosage('');
      setNewMedFrequency('');
    } catch (err) { console.error('Failed to add medication:', err); }
  };

  const removeMedication = async (id) => {
    try {
      await api.delete(`/subscribers/profile/medications/${id}`);
      setMedications(medications.filter(m => m.id !== id));
    } catch (err) { console.error('Failed to remove medication:', err); }
  };

  const addCondition = async () => {
    if (!newCondition.trim()) return;
    try {
      const res = await api.post('/subscribers/profile/conditions', { condition_name: newCondition.trim(), diagnosed_date: newCondDiagnosed || null, notes: newCondNotes || null });
      setConditions([...conditions, res.data]);
      setNewCondition('');
      setNewCondDiagnosed('');
      setNewCondNotes('');
    } catch (err) { console.error('Failed to add condition:', err); }
  };

  const removeCondition = async (id) => {
    try {
      await api.delete(`/subscribers/profile/conditions/${id}`);
      setConditions(conditions.filter(c => c.id !== id));
    } catch (err) { console.error('Failed to remove condition:', err); }
  };

  const validatePhone = (phone) => {
    const digits = phone.replace(/\D/g, '');
    if (digits.length === 0) return 'Phone number must contain digits';
    if (digits.length < 7) return 'Phone number is too short (minimum 7 digits)';
    return '';
  };

  const addContact = async () => {
    if (!newContactName.trim() || !newContactPhone.trim()) return;
    const phoneErr = validatePhone(newContactPhone);
    if (phoneErr) {
      setContactPhoneError(phoneErr);
      return;
    }
    setContactPhoneError('');
    try {
      const res = await api.post('/subscribers/profile/emergency-contacts', { name: newContactName.trim(), phone: newContactPhone.trim(), relationship: newContactRelationship || null });
      setEmergencyContacts([...emergencyContacts, res.data]);
      setNewContactName('');
      setNewContactPhone('');
      setNewContactRelationship('');
    } catch (err) { console.error('Failed to add contact:', err); }
  };

  const removeContact = async (id) => {
    try {
      await api.delete(`/subscribers/profile/emergency-contacts/${id}`);
      setEmergencyContacts(emergencyContacts.filter(c => c.id !== id));
    } catch (err) { console.error('Failed to remove contact:', err); }
  };

  const startEditContact = (contact) => {
    setEditingContactId(contact.id);
    setEditContactName(contact.name);
    setEditContactPhone(contact.phone);
    setEditContactRelationship(contact.relationship || '');
  };

  const cancelEditContact = () => {
    setEditingContactId(null);
    setEditContactName('');
    setEditContactPhone('');
    setEditContactRelationship('');
    setEditContactPhoneError('');
  };

  const saveEditContact = async (id) => {
    if (!editContactName.trim() || !editContactPhone.trim()) return;
    const phoneErr = validatePhone(editContactPhone);
    if (phoneErr) {
      setEditContactPhoneError(phoneErr);
      return;
    }
    setEditContactPhoneError('');
    try {
      const res = await api.put(`/subscribers/profile/emergency-contacts/${id}`, {
        name: editContactName.trim(),
        phone: editContactPhone.trim(),
        relationship: editContactRelationship || null,
      });
      setEmergencyContacts(emergencyContacts.map(c => c.id === id ? res.data : c));
      cancelEditContact();
    } catch (err) { console.error('Failed to edit contact:', err); }
  };

  const requestPhoneVerification = async () => {
    if (!phoneInput.trim()) return;
    setPhoneSaving(true);
    setPhoneMessage('');
    try {
      const res = await api.post('/subscribers/phone/request', { phone: phoneInput.trim() });
      setPhonePending(true);
      setPhoneDevCode(res.data.dev_code || '');
      setPhoneMessage('Verification code sent! Check your phone (or see the dev_code below).');
    } catch (err) {
      setPhoneMessage(err.response?.data?.error || 'Failed to send verification code');
    } finally {
      setPhoneSaving(false);
    }
  };

  const confirmPhoneVerification = async () => {
    if (!phoneVerificationCode.trim()) return;
    setPhoneSaving(true);
    setPhoneMessage('');
    try {
      const res = await api.post('/subscribers/phone/confirm', { code: phoneVerificationCode.trim() });
      setPhoneVerified(true);
      setPhone(phoneInput.trim());
      setPhonePending(false);
      setPhoneDevCode('');
      setPhoneVerificationCode('');
      setPhoneMessage(`✅ Phone verified! ${res.data.identity_core_points_awarded > 0 ? '+10 identity_core points awarded.' : 'Already claimed.'}`);
    } catch (err) {
      setPhoneMessage(err.response?.data?.error || 'Verification failed');
    } finally {
      setPhoneSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      {/* Unsaved Changes Dialog */}
      {blocker.state === 'blocked' && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50"
          role="dialog"
          aria-modal="true"
          aria-labelledby="unsaved-dialog-title"
          ref={unsavedDialogRef}
          onKeyDown={(e) => {
            if (!unsavedDialogRef.current) return;
            const focusable = unsavedDialogRef.current.querySelectorAll(
              'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
            );
            if (focusable.length === 0) return;
            const first = focusable[0];
            const last = focusable[focusable.length - 1];
            if (e.key === 'Tab') {
              if (e.shiftKey && document.activeElement === first) {
                e.preventDefault();
                last.focus();
              } else if (!e.shiftKey && document.activeElement === last) {
                e.preventDefault();
                first.focus();
              }
            }
          }}
        >
          <div className="bg-white rounded-xl shadow-2xl p-6 max-w-sm w-full mx-4">
            <h3 id="unsaved-dialog-title" className="text-lg font-semibold text-gray-900 mb-2">Unsaved Changes</h3>
            <p className="text-gray-600 mb-6">You have unsaved changes. If you leave this page, your changes will be lost.</p>
            <div className="flex gap-3 justify-end">
              <button
                onClick={() => blocker.reset()}
                className="bg-sky-500 hover:bg-sky-600 text-white font-medium py-2 px-4 rounded-lg transition-colors"
                data-testid="unsaved-stay-btn"
              >
                Stay on Page
              </button>
              <button
                onClick={() => blocker.proceed()}
                className="bg-gray-200 hover:bg-gray-300 text-gray-700 font-medium py-2 px-4 rounded-lg transition-colors"
                data-testid="unsaved-leave-btn"
              >
                Leave Page
              </button>
            </div>
          </div>
        </div>
      )}

      <main className="max-w-4xl mx-auto px-4 py-6">
        <h1 className="text-2xl font-bold text-gray-900 mb-6">Health Profile</h1>

        {message && (
          <div
            role="status"
            aria-live="polite"
            data-testid="profile-save-toast"
            className={`mb-4 p-3 rounded-lg text-sm flex items-center justify-between shadow-sm
              ${message.includes('success') ? 'bg-green-100 text-green-700 border border-green-200' : 'bg-red-100 text-red-700 border border-red-200'}`}
          >
            <span>
              {message.includes('success') ? '✅ ' : '⚠️ '}
              {message}
            </span>
            <button
              onClick={() => setMessage('')}
              className="ml-3 text-current opacity-60 hover:opacity-100 text-lg leading-none font-bold"
              aria-label="Dismiss notification"
              data-testid="dismiss-toast-btn"
            >
              ×
            </button>
          </div>
        )}

        {/* Basic Info Form */}
        <form onSubmit={handleSaveProfile} className="bg-white rounded-lg shadow p-6 mb-6" noValidate>
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Basic Information</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label htmlFor="profile-first-name" className="block text-sm font-medium text-gray-700 mb-1">First Name <span className="text-red-500">*</span></label>
              <input id="profile-first-name" type="text" value={firstName} onChange={e => setFirstName(e.target.value)}
                data-testid="profile-first-name"
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                placeholder="John" />
            </div>
            <div>
              <label htmlFor="profile-last-name" className="block text-sm font-medium text-gray-700 mb-1">Last Name <span className="text-red-500">*</span></label>
              <input id="profile-last-name" type="text" value={lastName} onChange={e => setLastName(e.target.value)}
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                placeholder="Doe" />
            </div>
            <div>
              <label htmlFor="profile-dob" className="block text-sm font-medium text-gray-700 mb-1">Date of Birth</label>
              <input
                id="profile-dob"
                type="date"
                value={dateOfBirth}
                onChange={e => setDateOfBirth(e.target.value)}
                min="1924-01-01"
                max={new Date().toISOString().split('T')[0]}
                data-testid="dob-date-input"
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
              />
              <p className="text-xs text-gray-400 mt-1">Date picker defaults to current date range</p>
            </div>
            <div>
              <label htmlFor="profile-blood-type" className="block text-sm font-medium text-gray-700 mb-1">Blood Type <span className="text-red-500">*</span></label>
              <select id="profile-blood-type" value={bloodType} onChange={e => setBloodType(e.target.value)}
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500">
                {bloodTypes.map(bt => <option key={bt} value={bt}>{bt || 'Select blood type'}</option>)}
              </select>
            </div>
            <div>
              <label htmlFor="profile-dnr-status" className="block text-sm font-medium text-gray-700 mb-1">DNR / Advance Directive Status</label>
              <select id="profile-dnr-status" value={dnrStatus} onChange={e => setDnrStatus(e.target.value)}
                className="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-sky-500 focus:border-sky-500">
                {DNR_OPTIONS.map(opt => (
                  <option key={opt} value={opt}>
                    {opt === 'not_specified' ? 'Not Specified' :
                     opt === 'full_code' ? 'Full Code' :
                     opt === 'dnr' ? 'Do Not Resuscitate (DNR)' :
                     opt === 'dnr_comfort_only' ? 'DNR - Comfort Care Only' :
                     'Limited Intervention'}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex items-center pt-6">
              <input type="checkbox" id="organDonor" checked={organDonor} onChange={e => setOrganDonor(e.target.checked)}
                className="h-4 w-4 text-sky-600 focus:ring-sky-500 border-gray-300 rounded" />
              <label htmlFor="organDonor" className="ml-2 text-sm text-gray-700">Organ Donor</label>
            </div>
          </div>
          <div className="mt-6">
            <button type="submit" disabled={saving}
              data-testid="profile-save-btn"
              aria-busy={saving}
              className="bg-sky-500 hover:bg-sky-600 text-white font-medium py-2 px-6 rounded-lg transition-colors disabled:opacity-50">
              {saving ? 'Saving...' : 'Save Profile'}
            </button>
          </div>
        </form>

        {/* Allergies Section */}
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Allergies</h2>
          {allergies.length > 0 && (
            <ul className="mb-4 space-y-2">
              {allergies.map(a => (
                <li key={a.id} className="flex items-center justify-between bg-red-50 rounded-lg px-3 py-2">
                  <span><strong>{a.allergy}</strong>{a.severity ? ` — ${a.severity}` : ''}</span>
                  <button onClick={() => removeAllergy(a.id)} className="text-red-500 hover:text-red-700 text-sm">Remove</button>
                </li>
              ))}
            </ul>
          )}
          <div className="grid grid-cols-1 sm:grid-cols-[1fr_1fr_auto] gap-2">
            <input type="text" value={newAllergy} onChange={e => setNewAllergy(e.target.value)}
              placeholder="Allergy name" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <input type="text" value={newAllergySeverity} onChange={e => setNewAllergySeverity(e.target.value)}
              placeholder="Severity (mild/moderate/severe)" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <button type="button" onClick={addAllergy}
              className="w-full sm:w-auto bg-red-500 hover:bg-red-600 text-white font-medium py-2 px-4 rounded-lg text-sm">Add Allergy</button>
          </div>
        </div>

        {/* Medications Section */}
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Medications</h2>
          {medications.length > 0 && (
            <ul className="mb-4 space-y-2">
              {medications.map(m => (
                <li key={m.id} className="flex items-center justify-between bg-blue-50 rounded-lg px-3 py-2">
                  <span><strong>{m.medication}</strong>{m.dosage ? ` — ${m.dosage}` : ''}{m.frequency ? ` (${m.frequency})` : ''}</span>
                  <button onClick={() => removeMedication(m.id)} className="text-red-500 hover:text-red-700 text-sm">Remove</button>
                </li>
              ))}
            </ul>
          )}
          <div className="grid grid-cols-1 sm:grid-cols-[2fr_1fr_1fr_auto] gap-2">
            <input type="text" value={newMedication} onChange={e => setNewMedication(e.target.value)}
              placeholder="Medication name" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <input type="text" value={newMedDosage} onChange={e => setNewMedDosage(e.target.value)}
              placeholder="Dosage" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <input type="text" value={newMedFrequency} onChange={e => setNewMedFrequency(e.target.value)}
              placeholder="Frequency" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <button type="button" onClick={addMedication}
              className="w-full sm:w-auto bg-blue-500 hover:bg-blue-600 text-white font-medium py-2 px-4 rounded-lg text-sm">Add Medication</button>
          </div>
        </div>

        {/* Conditions Section */}
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Medical Conditions</h2>
          {conditions.length > 0 && (
            <ul className="mb-4 space-y-2">
              {conditions.map(c => (
                <li key={c.id} className="flex items-center justify-between bg-amber-50 rounded-lg px-3 py-2">
                  <span>
                    <strong>{c.condition_name}</strong>
                    {c.diagnosed_date ? ` — diagnosed ${c.diagnosed_date.split('T')[0]}` : ''}
                    {c.notes ? ` (${c.notes})` : ''}
                  </span>
                  <button onClick={() => removeCondition(c.id)} className="text-red-500 hover:text-red-700 text-sm">Remove</button>
                </li>
              ))}
            </ul>
          )}
          <div className="grid grid-cols-1 sm:grid-cols-[2fr_1fr_1fr_auto] gap-2">
            <input type="text" value={newCondition} onChange={e => setNewCondition(e.target.value)}
              placeholder="Condition name" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <input type="date" value={newCondDiagnosed} onChange={e => setNewCondDiagnosed(e.target.value)}
              className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <input type="text" value={newCondNotes} onChange={e => setNewCondNotes(e.target.value)}
              placeholder="Notes" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <button type="button" onClick={addCondition}
              className="w-full sm:w-auto bg-amber-500 hover:bg-amber-600 text-white font-medium py-2 px-4 rounded-lg text-sm">Add Condition</button>
          </div>
        </div>

        {/* Phone Verification Section */}
        <div className="bg-white rounded-lg shadow p-6 mb-6" data-testid="phone-verification-section">
          <h2 className="text-lg font-semibold text-gray-900 mb-1">Phone Verification</h2>
          <p className="text-sm text-gray-500 mb-4">Verify your phone number to earn +10 identity_core points in your 0dentity score.</p>
          {phoneVerified ? (
            <div className="flex items-center gap-2 text-green-700 bg-green-50 rounded-lg px-3 py-2" data-testid="phone-verified-badge">
              <span>✅</span>
              <span className="font-medium">Phone verified: {phone}</span>
            </div>
          ) : (
            <div>
              <div className="flex flex-col sm:flex-row gap-2 mb-2">
                <input
                  type="tel"
                  value={phoneInput}
                  onChange={e => setPhoneInput(e.target.value)}
                  placeholder="e.g. +1 555-123-4567"
                  className="flex-1 border border-gray-300 rounded-lg px-3 py-2 text-sm"
                  data-testid="phone-input"
                />
                <button
                  type="button"
                  onClick={requestPhoneVerification}
                  disabled={phoneSaving || !phoneInput.trim()}
                  className="w-full sm:w-auto bg-sky-500 hover:bg-sky-600 text-white font-medium py-2 px-4 rounded-lg text-sm disabled:opacity-50"
                  data-testid="send-code-btn"
                >
                  {phoneSaving && !phonePending ? 'Sending...' : 'Send Code'}
                </button>
              </div>
              {phonePending && (
                <div className="mt-2">
                  {phoneDevCode && (
                    <div className="text-xs text-gray-500 bg-gray-50 rounded px-2 py-1 mb-2 font-mono" data-testid="dev-code">
                      Dev code: <strong>{phoneDevCode}</strong>
                    </div>
                  )}
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={phoneVerificationCode}
                      onChange={e => setPhoneVerificationCode(e.target.value)}
                      placeholder="Enter 6-digit code"
                      maxLength={6}
                      className="w-40 border border-gray-300 rounded-lg px-3 py-2 text-sm font-mono"
                      data-testid="verification-code-input"
                    />
                    <button
                      type="button"
                      onClick={confirmPhoneVerification}
                      disabled={phoneSaving || !phoneVerificationCode.trim()}
                      className="bg-green-500 hover:bg-green-600 text-white font-medium py-2 px-4 rounded-lg text-sm disabled:opacity-50"
                      data-testid="confirm-code-btn"
                    >
                      {phoneSaving && phonePending ? 'Verifying...' : 'Verify Code'}
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
          {phoneMessage && (
            <p className={`mt-2 text-sm ${phoneMessage.startsWith('✅') ? 'text-green-600' : 'text-red-600'}`} data-testid="phone-message">
              {phoneMessage}
            </p>
          )}
        </div>

        {/* Emergency Contacts Section */}
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Emergency Contacts</h2>
          {emergencyContacts.length > 0 && (
            <ul className="mb-4 space-y-2">
              {emergencyContacts.map(c => (
                <li key={c.id} className="bg-green-50 rounded-lg px-3 py-2">
                  {editingContactId === c.id ? (
                    <div className="flex flex-wrap gap-2 items-start">
                      <input type="text" value={editContactName} onChange={e => setEditContactName(e.target.value)}
                        placeholder="Name" className="flex-1 min-w-[120px] border border-gray-300 rounded-lg px-3 py-1 text-sm" />
                      <div className="flex flex-col">
                        <input type="tel" value={editContactPhone} onChange={e => { setEditContactPhone(e.target.value); setEditContactPhoneError(''); }}
                          placeholder="Phone" className={`w-36 border rounded-lg px-3 py-1 text-sm ${editContactPhoneError ? 'border-red-500' : 'border-gray-300'}`} />
                        {editContactPhoneError && <span className="text-red-600 text-xs mt-1" role="alert">{editContactPhoneError}</span>}
                      </div>
                      <input type="text" value={editContactRelationship} onChange={e => setEditContactRelationship(e.target.value)}
                        placeholder="Relationship" className="w-32 border border-gray-300 rounded-lg px-3 py-1 text-sm" />
                      <button onClick={() => saveEditContact(c.id)}
                        className="bg-emerald-500 hover:bg-emerald-600 text-white text-sm px-3 py-1 rounded-lg">Save</button>
                      <button onClick={cancelEditContact}
                        className="bg-gray-200 hover:bg-gray-300 text-gray-700 text-sm px-3 py-1 rounded-lg">Cancel</button>
                    </div>
                  ) : (
                    <div className="flex items-center justify-between">
                      <span><strong>{c.name}</strong> — {c.phone}{c.relationship ? ` (${c.relationship})` : ''}</span>
                      <div className="flex gap-2">
                        <button onClick={() => startEditContact(c)}
                          className="text-blue-500 hover:text-blue-700 text-sm">Edit</button>
                        <button onClick={() => removeContact(c.id)}
                          className="text-red-500 hover:text-red-700 text-sm">Remove</button>
                      </div>
                    </div>
                  )}
                </li>
              ))}
            </ul>
          )}
          <div className="grid grid-cols-1 sm:grid-cols-[2fr_1fr_1fr_auto] gap-2 items-start">
            <input type="text" value={newContactName} onChange={e => setNewContactName(e.target.value)}
              placeholder="Contact name" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <div className="flex flex-col">
              <input type="tel" value={newContactPhone} onChange={e => { setNewContactPhone(e.target.value); setContactPhoneError(''); }}
                placeholder="Phone number" className={`w-full border rounded-lg px-3 py-2 text-sm ${contactPhoneError ? 'border-red-500' : 'border-gray-300'}`} />
              {contactPhoneError && <span className="text-red-600 text-xs mt-1" role="alert">{contactPhoneError}</span>}
            </div>
            <input type="text" value={newContactRelationship} onChange={e => setNewContactRelationship(e.target.value)}
              placeholder="Relationship" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm" />
            <button type="button" onClick={addContact}
              className="w-full sm:w-auto bg-emerald-500 hover:bg-emerald-600 text-white font-medium py-2 px-4 rounded-lg text-sm">Add Contact</button>
          </div>
        </div>
      </main>
    </div>
  );
}
