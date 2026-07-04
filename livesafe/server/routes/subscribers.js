const express = require('express');
const router = express.Router();
const jwt = require('jsonwebtoken');
const { buildInactiveDeletionAuditMetadata } = require('../utils/deletion-audit-metadata');
const {
  buildAdminSubscriberListResponse,
  buildAdminSubscriberResponse,
} = require('../utils/admin-subscriber-response');
const {
  buildPublicAlertSettingsMutationResponse,
  buildPublicAlertSettingsResponse,
  buildPublicConsentDefaultsMutationResponse,
  buildPublicConsentDefaultsResponse,
  buildPublicEmergencyContactResponse,
  buildPublicSubscriberDeleteAcknowledgement,
  buildPublicSubscriberAllergyWriteResponse,
  buildPublicSubscriberConditionWriteResponse,
  buildPublicSubscriberMedicationWriteResponse,
  buildPublicSubscriberProfileResponse,
  buildPublicSubscriberProfileSummary,
} = require('../utils/subscriber-profile-response');
const {
  buildPublicPhoneVerificationRequestResponse,
  buildPublicPhoneVerificationConfirmResponse,
} = require('../utils/verification-response');

const JWT_SECRET = process.env.JWT_SECRET;

// Auth middleware for profile routes (subscriber-only)
function authMiddleware(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'No token provided' });
  }
  try {
    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);
    // Enforce subscriber-only access — responders and providers cannot access subscriber profile endpoints
    if (decoded.role !== 'subscriber') {
      return res.status(403).json({
        error: 'Access denied: subscriber profile endpoints require subscriber authentication',
        code: 'SUBSCRIBER_ONLY',
        access_type: 'none',
      });
    }
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

// General auth middleware — requires any valid JWT token (any role)
function requireAnyAuth(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  try {
    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

function adminOnly(req, res, next) {
  if (!req.user || req.user.role !== 'subscriber_admin') {
    return res.status(403).json({ error: 'Admin access required' });
  }
  next();
}

// Email validation helper
function isValidEmail(email) {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

// Validate blood type
const VALID_BLOOD_TYPES = ['A+', 'A-', 'B+', 'B-', 'AB+', 'AB-', 'O+', 'O-'];

// Validate DNR status
const VALID_DNR_STATUSES = ['not_specified', 'full_code', 'dnr', 'dnr_comfort', 'limited'];

// POST /api/subscribers - Create a new subscriber
router.post('/', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password, first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor } = req.body;

    // Server-side validation: collect field-level errors
    const errors = {};

    // Required fields
    if (!email || (typeof email === 'string' && email.trim() === '')) {
      errors.email = 'Email is required';
    } else if (!isValidEmail(email)) {
      errors.email = 'Invalid email format';
    }

    if (!password || (typeof password === 'string' && password.trim() === '')) {
      errors.password = 'Password is required';
    } else if (typeof password === 'string' && password.length < 6) {
      errors.password = 'Password must be at least 6 characters';
    }

    // Optional field validation
    if (blood_type && !VALID_BLOOD_TYPES.includes(blood_type)) {
      errors.blood_type = 'Invalid blood type. Must be one of: ' + VALID_BLOOD_TYPES.join(', ');
    }

    if (dnr_status && !VALID_DNR_STATUSES.includes(dnr_status)) {
      errors.dnr_status = 'Invalid DNR status. Must be one of: ' + VALID_DNR_STATUSES.join(', ');
    }

    if (date_of_birth) {
      const dob = new Date(date_of_birth);
      if (isNaN(dob.getTime())) {
        errors.date_of_birth = 'Invalid date format for date of birth';
      }
    }

    if (Object.keys(errors).length > 0) {
      return res.status(400).json({ error: 'Validation failed', errors });
    }

    const { v4: uuidv4 } = require('uuid');
    const bcrypt = require('bcryptjs');

    const did = `did:exo:subscriber:${uuidv4()}`;
    const password_hash = await bcrypt.hash(password, 10);

    const result = await db.query(
      `INSERT INTO subscribers (did, email, password_hash, first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
       RETURNING id, did, email, first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor, role, created_at`,
      [did, email, password_hash, first_name || null, last_name || null, date_of_birth || null, blood_type || null, dnr_status || 'not_specified', organ_donor || false]
    );

    res.status(201).json(buildAdminSubscriberResponse(result.rows[0]));
  } catch (err) {
    if (err.code === '23505') {
      return res.status(409).json({ error: 'Email already exists' });
    }
    console.error('[Subscribers] Create error:', err.message);
    res.status(500).json({ error: 'Failed to create subscriber' });
  }
});

// GET /api/subscribers - Legacy subscriber-management alias (admin only)
router.get('/', requireAnyAuth, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      'SELECT id, email, first_name, last_name, role, email_verified, created_at, updated_at FROM subscribers ORDER BY created_at DESC'
    );
    res.json(buildAdminSubscriberListResponse(result.rows));
  } catch (err) {
    console.error('[Subscribers] List error:', err.message);
    res.status(500).json({ error: 'Failed to list subscribers' });
  }
});

// GET /api/subscribers/blood-types - Return valid blood type options
router.get('/blood-types', (req, res) => {
  res.json({ blood_types: VALID_BLOOD_TYPES });
});

// =============================================================================
// PROFILE ROUTES (authenticated) - MUST be before /:did to avoid conflict
// =============================================================================

// GET /api/subscribers/vault-completeness - Calculate vault completeness score
// Score = 5 sections × 20% each:
//   1. Profile (blood_type AND date_of_birth filled)
//   2. Allergies (≥1 entry)
//   3. Medications (≥1 entry)
//   4. Conditions (≥1 entry)
//   5. Insurance card (≥1 credential of type 'insurance_card')
router.get('/vault-completeness', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    // Check profile completeness (blood_type and date_of_birth)
    const profileResult = await db.query(
      'SELECT blood_type, date_of_birth FROM subscribers WHERE id = $1',
      [subscriberId]
    );
    const sub = profileResult.rows[0] || {};
    const profileComplete = !!(sub.blood_type && sub.date_of_birth);

    // Check allergies
    const allergyResult = await db.query(
      'SELECT COUNT(*) AS cnt FROM subscriber_allergies WHERE subscriber_id = $1',
      [subscriberId]
    );
    const hasAllergies = parseInt(allergyResult.rows[0].cnt) > 0;

    // Check medications
    const medResult = await db.query(
      'SELECT COUNT(*) AS cnt FROM subscriber_medications WHERE subscriber_id = $1',
      [subscriberId]
    );
    const hasMedications = parseInt(medResult.rows[0].cnt) > 0;

    // Check conditions
    const condResult = await db.query(
      'SELECT COUNT(*) AS cnt FROM subscriber_conditions WHERE subscriber_id = $1',
      [subscriberId]
    );
    const hasConditions = parseInt(condResult.rows[0].cnt) > 0;

    // Check insurance card in credentials table
    const insuranceResult = await db.query(
      "SELECT COUNT(*) AS cnt FROM credentials WHERE subscriber_id = $1 AND credential_type = 'insurance_card'",
      [subscriberId]
    );
    const hasInsurance = parseInt(insuranceResult.rows[0].cnt) > 0;

    const sections = {
      profile: profileComplete,
      allergies: hasAllergies,
      medications: hasMedications,
      conditions: hasConditions,
      insurance_card: hasInsurance,
    };

    const filledSections = Object.values(sections).filter(Boolean).length;
    const score = Math.round((filledSections / 5) * 100);

    res.json({
      score,
      sections,
      sections_filled: filledSections,
      sections_total: 5,
      section_weight: 20,
    });
  } catch (err) {
    console.error('[Subscribers] Vault completeness error:', err.message);
    res.status(500).json({ error: 'Failed to calculate vault completeness' });
  }
});

// GET /api/subscribers/profile - Get current subscriber's full profile
router.get('/profile', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    const subResult = await db.query(
      'SELECT id, did, email, first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor, role, email_verified, alert_sensitivity, phone, phone_verified, created_at FROM subscribers WHERE id = $1',
      [subscriberId]
    );

    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const subscriber = subResult.rows[0];

    const allergiesResult = await db.query(
      'SELECT id, allergy, severity, created_at FROM subscriber_allergies WHERE subscriber_id = $1 ORDER BY created_at',
      [subscriberId]
    );

    const medsResult = await db.query(
      'SELECT id, medication, dosage, frequency, created_at FROM subscriber_medications WHERE subscriber_id = $1 ORDER BY created_at',
      [subscriberId]
    );

    const condResult = await db.query(
      'SELECT id, condition_name, diagnosed_date, notes, created_at FROM subscriber_conditions WHERE subscriber_id = $1 ORDER BY created_at',
      [subscriberId]
    );

    const contactsResult = await db.query(
      'SELECT id, name, phone, relationship, created_at FROM emergency_contacts WHERE subscriber_id = $1 ORDER BY created_at',
      [subscriberId]
    );

    res.json(buildPublicSubscriberProfileResponse({
      subscriber,
      allergies: allergiesResult.rows,
      medications: medsResult.rows,
      conditions: condResult.rows,
      emergencyContacts: contactsResult.rows,
    }));
  } catch (err) {
    console.error('[Subscribers] Profile error:', err.message);
    res.status(500).json({ error: 'Failed to get profile' });
  }
});

// PUT /api/subscribers/profile - Update profile fields
router.put('/profile', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor } = req.body;

    const result = await db.query(
      `UPDATE subscribers SET
        first_name = COALESCE($1, first_name),
        last_name = COALESCE($2, last_name),
        date_of_birth = $3,
        blood_type = $4,
        dnr_status = COALESCE($5, dnr_status),
        organ_donor = COALESCE($6, organ_donor),
        updated_at = NOW()
      WHERE id = $7
      RETURNING id, did, email, first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor, role, email_verified`,
      [first_name || null, last_name || null, date_of_birth || null, blood_type || null, dnr_status || 'not_specified', organ_donor !== undefined ? organ_donor : false, subscriberId]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json(buildPublicSubscriberProfileSummary(result.rows[0]));
  } catch (err) {
    console.error('[Subscribers] Profile update error:', err.message);
    res.status(500).json({ error: 'Failed to update profile' });
  }
});

// POST /api/subscribers/profile/allergies - Add allergy
router.post('/profile/allergies', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { allergy, severity } = req.body;
    if (!allergy) return res.status(400).json({ error: 'Allergy name is required' });

    const result = await db.query(
      'INSERT INTO subscriber_allergies (subscriber_id, allergy, severity) VALUES ($1, $2, $3) RETURNING *',
      [req.user.id, allergy, severity || null]
    );

    // Award 0dentity health_record_completeness +15 points on first allergy entry
    const existingClaim = await db.query(
      "SELECT id FROM odentity_claims WHERE subscriber_id = $1 AND claim_type = 'allergies_entered' AND dimension = 'health_record_completeness'",
      [req.user.id]
    );
    let odentity_claim = null;
    if (existingClaim.rows.length === 0) {
      const claimResult = await db.query(
        "INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer) VALUES ($1, 'allergies_entered', 'health_record_completeness', 15, 'livesafe') RETURNING *",
        [req.user.id]
      );
      await db.query(
        "INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count) VALUES ($1, 'health_record_completeness', 15, 1) ON CONFLICT (subscriber_id, dimension) DO UPDATE SET current_score = LEAST(odentity_scores.current_score + 15, odentity_scores.max_possible), claim_count = odentity_scores.claim_count + 1, last_updated = NOW()",
        [req.user.id]
      );
      odentity_claim = claimResult.rows[0];
      console.log('[0dentity] Awarded allergies_entered claim: +15 health_record_completeness for subscriber', req.user.id);
    }

    res.status(201).json(buildPublicSubscriberAllergyWriteResponse({
      allergy: result.rows[0],
      odentityClaim: odentity_claim,
    }));
  } catch (err) {
    console.error('[Subscribers] Add allergy error:', err.message);
    res.status(500).json({ error: 'Failed to add allergy' });
  }
});

// DELETE /api/subscribers/profile/allergies/:id - Remove allergy
router.delete('/profile/allergies/:id', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      'DELETE FROM subscriber_allergies WHERE id = $1 AND subscriber_id = $2 RETURNING id',
      [parseInt(req.params.id), req.user.id]
    );
    if (result.rows.length === 0) return res.status(404).json({ error: 'Allergy not found' });
    res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Allergy removed' }));
  } catch (err) {
    console.error('[Subscribers] Delete allergy error:', err.message);
    res.status(500).json({ error: 'Failed to remove allergy' });
  }
});

// POST /api/subscribers/profile/medications - Add medication
router.post('/profile/medications', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { medication, dosage, frequency } = req.body;
    if (!medication) return res.status(400).json({ error: 'Medication name is required' });

    const result = await db.query(
      'INSERT INTO subscriber_medications (subscriber_id, medication, dosage, frequency) VALUES ($1, $2, $3, $4) RETURNING *',
      [req.user.id, medication, dosage || null, frequency || null]
    );

    // Award 0dentity health_record_completeness +15 points on first medication entry
    const existingClaim = await db.query(
      "SELECT id FROM odentity_claims WHERE subscriber_id = $1 AND claim_type = 'medications_entered' AND dimension = 'health_record_completeness'",
      [req.user.id]
    );
    let odentity_claim = null;
    if (existingClaim.rows.length === 0) {
      const claimResult = await db.query(
        "INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer) VALUES ($1, 'medications_entered', 'health_record_completeness', 15, 'livesafe') RETURNING *",
        [req.user.id]
      );
      await db.query(
        "INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count) VALUES ($1, 'health_record_completeness', 15, 1) ON CONFLICT (subscriber_id, dimension) DO UPDATE SET current_score = LEAST(odentity_scores.current_score + 15, odentity_scores.max_possible), claim_count = odentity_scores.claim_count + 1, last_updated = NOW()",
        [req.user.id]
      );
      odentity_claim = claimResult.rows[0];
      console.log('[0dentity] Awarded medications_entered claim: +15 health_record_completeness for subscriber', req.user.id);
    }

    res.status(201).json(buildPublicSubscriberMedicationWriteResponse({
      medication: result.rows[0],
      odentityClaim: odentity_claim,
    }));
  } catch (err) {
    console.error('[Subscribers] Add medication error:', err.message);
    res.status(500).json({ error: 'Failed to add medication' });
  }
});

// DELETE /api/subscribers/profile/medications/:id - Remove medication
router.delete('/profile/medications/:id', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      'DELETE FROM subscriber_medications WHERE id = $1 AND subscriber_id = $2 RETURNING id',
      [parseInt(req.params.id), req.user.id]
    );
    if (result.rows.length === 0) return res.status(404).json({ error: 'Medication not found' });
    res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Medication removed' }));
  } catch (err) {
    console.error('[Subscribers] Delete medication error:', err.message);
    res.status(500).json({ error: 'Failed to remove medication' });
  }
});

// POST /api/subscribers/profile/conditions - Add condition
router.post('/profile/conditions', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { condition_name, diagnosed_date, notes } = req.body;
    if (!condition_name) return res.status(400).json({ error: 'Condition name is required' });

    const result = await db.query(
      'INSERT INTO subscriber_conditions (subscriber_id, condition_name, diagnosed_date, notes) VALUES ($1, $2, $3, $4) RETURNING *',
      [req.user.id, condition_name, diagnosed_date || null, notes || null]
    );

    // Award 0dentity health_record_completeness +15 points on first condition entry
    const existingClaim = await db.query(
      "SELECT id FROM odentity_claims WHERE subscriber_id = $1 AND claim_type = 'conditions_entered' AND dimension = 'health_record_completeness'",
      [req.user.id]
    );
    let odentity_claim = null;
    if (existingClaim.rows.length === 0) {
      const claimResult = await db.query(
        "INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer) VALUES ($1, 'conditions_entered', 'health_record_completeness', 15, 'livesafe') RETURNING *",
        [req.user.id]
      );
      await db.query(
        "INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count) VALUES ($1, 'health_record_completeness', 15, 1) ON CONFLICT (subscriber_id, dimension) DO UPDATE SET current_score = LEAST(odentity_scores.current_score + 15, odentity_scores.max_possible), claim_count = odentity_scores.claim_count + 1, last_updated = NOW()",
        [req.user.id]
      );
      odentity_claim = claimResult.rows[0];
      console.log('[0dentity] Awarded conditions_entered claim: +15 health_record_completeness for subscriber', req.user.id);
    }

    res.status(201).json(buildPublicSubscriberConditionWriteResponse({
      condition: result.rows[0],
      odentityClaim: odentity_claim,
    }));
  } catch (err) {
    console.error('[Subscribers] Add condition error:', err.message);
    res.status(500).json({ error: 'Failed to add condition' });
  }
});

// DELETE /api/subscribers/profile/conditions/:id - Remove condition
router.delete('/profile/conditions/:id', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      'DELETE FROM subscriber_conditions WHERE id = $1 AND subscriber_id = $2 RETURNING id',
      [parseInt(req.params.id), req.user.id]
    );
    if (result.rows.length === 0) return res.status(404).json({ error: 'Condition not found' });
    res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Condition removed' }));
  } catch (err) {
    console.error('[Subscribers] Delete condition error:', err.message);
    res.status(500).json({ error: 'Failed to remove condition' });
  }
});

// POST /api/subscribers/profile/emergency-contacts - Add emergency contact
router.post('/profile/emergency-contacts', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { name, phone, relationship } = req.body;
    if (!name || !phone) return res.status(400).json({ error: 'Name and phone are required' });

    // Check if this is the first emergency contact (for 0dentity score update)
    const existingContacts = await db.query(
      'SELECT COUNT(*) FROM emergency_contacts WHERE subscriber_id = $1',
      [req.user.id]
    );
    const isFirstContact = parseInt(existingContacts.rows[0].count) === 0;

    const result = await db.query(
      'INSERT INTO emergency_contacts (subscriber_id, name, phone, relationship) VALUES ($1, $2, $3, $4) RETURNING *',
      [req.user.id, name, phone, relationship || null]
    );

    // Award 0dentity claim for entering emergency contacts (only once, first contact)
    if (isFirstContact) {
      try {
        // Check if claim already exists
        const existingClaim = await db.query(
          "SELECT id FROM odentity_claims WHERE subscriber_id = $1 AND claim_type = 'emergency_contacts_entered' AND revoked_at IS NULL",
          [req.user.id]
        );
        if (existingClaim.rows.length === 0) {
          await db.query(
            `INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer)
             VALUES ($1, 'emergency_contacts_entered', 'health_record_completeness', 20, 'livesafe')`,
            [req.user.id]
          );
          // Update health_record_completeness score
          await db.query(
            `INSERT INTO odentity_scores (subscriber_id, dimension, current_score, max_possible, claim_count)
             VALUES ($1, 'health_record_completeness', 20, 100, 1)
             ON CONFLICT (subscriber_id, dimension)
             DO UPDATE SET
               current_score = LEAST(odentity_scores.current_score + 20, odentity_scores.max_possible),
               claim_count = odentity_scores.claim_count + 1,
               last_updated = NOW()`,
            [req.user.id]
          );
          console.log('[0dentity] Awarded emergency_contacts_entered claim (+20 health_record_completeness) to subscriber', req.user.id);
        }
      } catch (scoreErr) {
        console.error('[0dentity] Failed to award emergency contacts score:', scoreErr.message);
        // Don't fail the whole request if scoring fails
      }
    }

    res.status(201).json(buildPublicEmergencyContactResponse(result.rows[0]));
  } catch (err) {
    console.error('[Subscribers] Add emergency contact error:', err.message);
    res.status(500).json({ error: 'Failed to add emergency contact' });
  }
});

// PUT /api/subscribers/profile/emergency-contacts/:id - Edit emergency contact
router.put('/profile/emergency-contacts/:id', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { name, phone, relationship } = req.body;
    if (!name || !phone) return res.status(400).json({ error: 'Name and phone are required' });

    const result = await db.query(
      'UPDATE emergency_contacts SET name = $1, phone = $2, relationship = $3 WHERE id = $4 AND subscriber_id = $5 RETURNING *',
      [name, phone, relationship || null, parseInt(req.params.id), req.user.id]
    );
    if (result.rows.length === 0) return res.status(404).json({ error: 'Emergency contact not found' });
    res.json(buildPublicEmergencyContactResponse(result.rows[0]));
  } catch (err) {
    console.error('[Subscribers] Edit emergency contact error:', err.message);
    res.status(500).json({ error: 'Failed to edit emergency contact' });
  }
});

// DELETE /api/subscribers/profile/emergency-contacts/:id - Remove emergency contact
router.delete('/profile/emergency-contacts/:id', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      'DELETE FROM emergency_contacts WHERE id = $1 AND subscriber_id = $2 RETURNING id',
      [parseInt(req.params.id), req.user.id]
    );
    if (result.rows.length === 0) return res.status(404).json({ error: 'Emergency contact not found' });
    res.json(buildPublicSubscriberDeleteAcknowledgement({ message: 'Emergency contact removed' }));
  } catch (err) {
    console.error('[Subscribers] Delete emergency contact error:', err.message);
    res.status(500).json({ error: 'Failed to remove emergency contact' });
  }
});

// GET /api/subscribers/alert-settings - Get current subscriber's alert sensitivity setting
// Feature #72, #280: Returns alert_sensitivity plus sms/push/email channel booleans
router.get('/alert-settings', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    const result = await db.query(
      'SELECT alert_sensitivity, sms_alerts, push_alerts, email_alerts FROM subscribers WHERE id = $1',
      [subscriberId]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json(buildPublicAlertSettingsResponse(result.rows[0]));
  } catch (err) {
    console.error('[Subscribers] Alert settings get error:', err.message);
    res.status(500).json({ error: 'Failed to get alert settings' });
  }
});

// PUT /api/subscribers/alert-settings - Update alert sensitivity and channel settings
// Feature #72, #280: Sets alert_sensitivity and sms/push/email channel booleans
router.put('/alert-settings', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { alert_sensitivity, sms_alerts, push_alerts, email_alerts } = req.body;

    const validOptions = ['always', 'emergency-only', 'off'];
    if (!alert_sensitivity || !validOptions.includes(alert_sensitivity)) {
      return res.status(400).json({
        error: `alert_sensitivity must be one of: ${validOptions.join(', ')}`,
        options: validOptions,
      });
    }

    // Build update with optional channel fields
    const smsVal = sms_alerts !== undefined ? Boolean(sms_alerts) : null;
    const pushVal = push_alerts !== undefined ? Boolean(push_alerts) : null;
    const emailVal = email_alerts !== undefined ? Boolean(email_alerts) : null;

    let query, params;
    if (smsVal !== null || pushVal !== null || emailVal !== null) {
      query = `UPDATE subscribers
               SET alert_sensitivity = $1,
                   sms_alerts = COALESCE($3, sms_alerts),
                   push_alerts = COALESCE($4, push_alerts),
                   email_alerts = COALESCE($5, email_alerts),
                   updated_at = NOW()
               WHERE id = $2
               RETURNING id, did, alert_sensitivity, sms_alerts, push_alerts, email_alerts`;
      params = [alert_sensitivity, subscriberId, smsVal, pushVal, emailVal];
    } else {
      query = `UPDATE subscribers SET alert_sensitivity = $1, updated_at = NOW() WHERE id = $2
               RETURNING id, did, alert_sensitivity, sms_alerts, push_alerts, email_alerts`;
      params = [alert_sensitivity, subscriberId];
    }

    const result = await db.query(query, params);

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const row = result.rows[0];
    console.log(`[Subscribers] Alert settings updated for subscriber ${subscriberId}: sensitivity=${alert_sensitivity}, sms=${row.sms_alerts}, push=${row.push_alerts}, email=${row.email_alerts}`);

    res.json(buildPublicAlertSettingsMutationResponse({
      ...row,
      message: 'Alert settings saved',
    }));
  } catch (err) {
    console.error('[Subscribers] Alert settings update error:', err.message);
    res.status(500).json({ error: 'Failed to update alert settings' });
  }
});

// GET /api/subscribers/consent-defaults - Get consent default scope and duration
// Feature #160: Returns default_scope and default_duration_days for new consent grants
router.get('/consent-defaults', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    const result = await db.query(
      `SELECT consent_default_scope, consent_default_duration_days
       FROM subscribers WHERE id = $1`,
      [subscriberId]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json(buildPublicConsentDefaultsResponse(result.rows[0]));
  } catch (err) {
    console.error('[Subscribers] Consent defaults get error:', err.message);
    res.status(500).json({ error: 'Failed to get consent defaults' });
  }
});

// PUT /api/subscribers/consent-defaults - Update consent default scope and duration
// Feature #160: Sets default_scope and default_duration_days
router.put('/consent-defaults', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { default_scope, default_duration_days } = req.body;

    const validScopes = ['basic_health', 'full_health', 'emergency_only', 'research'];
    const validDurations = [7, 30, 90, 180, 365];

    if (default_scope && !validScopes.includes(default_scope)) {
      return res.status(400).json({ error: `default_scope must be one of: ${validScopes.join(', ')}` });
    }

    if (default_duration_days !== undefined && !validDurations.includes(Number(default_duration_days))) {
      return res.status(400).json({ error: `default_duration_days must be one of: ${validDurations.join(', ')}` });
    }

    // Ensure columns exist (migration safety)
    await db.query(`
      ALTER TABLE subscribers
        ADD COLUMN IF NOT EXISTS consent_default_scope VARCHAR(50) DEFAULT 'basic_health',
        ADD COLUMN IF NOT EXISTS consent_default_duration_days INTEGER DEFAULT 30
    `).catch(() => {});

    const result = await db.query(
      `UPDATE subscribers
       SET consent_default_scope = COALESCE($1, consent_default_scope),
           consent_default_duration_days = COALESCE($2, consent_default_duration_days),
           updated_at = NOW()
       WHERE id = $3
       RETURNING id, consent_default_scope, consent_default_duration_days`,
      [default_scope || null, default_duration_days ? Number(default_duration_days) : null, subscriberId]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json(buildPublicConsentDefaultsMutationResponse({
      ...result.rows[0],
      message: 'Consent defaults updated successfully',
    }));
  } catch (err) {
    console.error('[Subscribers] Consent defaults update error:', err.message);
    res.status(500).json({ error: 'Failed to update consent defaults' });
  }
});

// =============================================================================
// GENERIC PARAM ROUTES (must be AFTER /profile routes)
// =============================================================================

// GET /api/subscribers/:did - Get subscriber by DID
// Feature #175: Protected endpoint — requires auth header (any role)
// Feature #161: If requesting as a subscriber, only allow access to own profile (403 for cross-subscriber)
// Feature #253: Malformed DIDs return 404 gracefully
router.get('/:did', requireAnyAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { did } = req.params;

    // Feature #253: Validate DID/ID format before doing anything else
    // Valid formats: starts with 'did:' OR is a positive integer
    const isDidFormat = did.startsWith('did:');
    const parsedId = parseInt(did, 10);
    const isIntegerFormat = !isNaN(parsedId) && String(parsedId) === did.trim() && parsedId > 0;

    if (!isDidFormat && !isIntegerFormat) {
      // Malformed identifier — no subscriber will ever have this ID, return 404 gracefully
      return res.status(404).json({
        error: 'Subscriber not found',
        code: 'INVALID_IDENTIFIER',
        message: 'The identifier format is not valid. Expected a DID (did:exo:subscriber:...) or numeric ID.',
      });
    }

    // Feature #161: Check for subscriber JWT — if present, enforce data isolation
    const authHeader = req.headers.authorization;
    if (authHeader && authHeader.startsWith('Bearer ')) {
      try {
        const token = authHeader.split(' ')[1];
        const decoded = jwt.verify(token, JWT_SECRET);
        if (decoded.role === 'subscriber') {
          // Subscriber role: only allowed to fetch their own profile
          const requestedDid = did;
          // If DID-based lookup, compare against token DID
          if (isDidFormat && decoded.did && decoded.did !== requestedDid) {
            return res.status(403).json({
              error: 'Forbidden: subscribers cannot access another subscriber\'s profile',
              code: 'CROSS_SUBSCRIBER_ACCESS_DENIED',
            });
          }
          // If ID-based lookup, compare against token ID
          if (isIntegerFormat && decoded.id && parsedId !== decoded.id) {
            return res.status(403).json({
              error: 'Forbidden: subscribers cannot access another subscriber\'s profile',
              code: 'CROSS_SUBSCRIBER_ACCESS_DENIED',
            });
          }
        }
      } catch (jwtErr) {
        // Invalid JWT: allow fallthrough to public access (unauthenticated request)
      }
    }

    let result;
    if (isDidFormat) {
      result = await db.query(
        'SELECT id, did, email, first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor, role, created_at FROM subscribers WHERE did = $1',
        [did]
      );
    } else {
      result = await db.query(
        'SELECT id, did, email, first_name, last_name, date_of_birth, blood_type, dnr_status, organ_donor, role, created_at FROM subscribers WHERE id = $1',
        [parsedId]
      );
    }

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json(buildAdminSubscriberResponse(result.rows[0]));
  } catch (err) {
    console.error('[Subscribers] Get error:', err.message);
    res.status(500).json({ error: 'Failed to get subscriber' });
  }
});

// PATCH /api/subscribers/:did - Legacy subscriber account-management alias (admin only)
router.patch('/:did', requireAnyAuth, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { did } = req.params;
    const updates = req.body;

    const allowedFields = ['role', 'email_verified'];
    const setClauses = [];
    const values = [];
    let paramIndex = 1;

    for (const [key, value] of Object.entries(updates)) {
      if (allowedFields.includes(key)) {
        setClauses.push(`${key} = $${paramIndex}`);
        values.push(value);
        paramIndex++;
      }
    }

    if (setClauses.length === 0) {
      return res.status(400).json({ error: 'No valid fields to update' });
    }

    setClauses.push(`updated_at = NOW()`);
    values.push(did);

    const query = `UPDATE subscribers SET ${setClauses.join(', ')} WHERE did = $${paramIndex} RETURNING id, email, first_name, last_name, role, email_verified, created_at, updated_at`;
    const result = await db.query(query, values);

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json(buildAdminSubscriberResponse(result.rows[0]));
  } catch (err) {
    console.error('[Subscribers] Update error:', err.message);
    res.status(500).json({ error: 'Failed to update subscriber' });
  }
});

// DELETE /api/subscribers/account - Delete own account (authenticated, requires password confirmation)
// Must be BEFORE /:id to prevent "account" matching /:id
router.delete('/account', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const bcrypt = require('bcryptjs');
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;
    const { password } = req.body;

    if (!password) {
      return res.status(400).json({ error: 'Password confirmation is required to delete your account' });
    }

    // Verify password
    const subResult = await db.query('SELECT id, email, did, password_hash FROM subscribers WHERE id = $1', [subscriberId]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];

    const passwordMatch = await bcrypt.compare(password, subscriber.password_hash);
    if (!passwordMatch) {
      return res.status(400).json({ error: 'Incorrect password. Account deletion cancelled.' });
    }

    // Count associated data before deletion (for audit record)
    const [recordsCount, credentialsCount, trusteesCount, consentsCount] = await Promise.all([
      db.query('SELECT COUNT(*) FROM medical_records WHERE subscriber_id = $1', [subscriberId]),
      db.query('SELECT COUNT(*) FROM credentials WHERE subscriber_id = $1', [subscriberId]),
      db.query('SELECT COUNT(*) FROM trustees WHERE subscriber_id = $1', [subscriberId]),
      db.query('SELECT COUNT(*) FROM consent_events WHERE subscriber_id = $1', [subscriberId]),
    ]);

    // Create audit receipt BEFORE deletion (preserved via DID reference, no FK)
    const crypto = require('crypto');
    const deletionTimestamp = new Date().toISOString();
    const prevHashRow = await db.query(
      'SELECT receipt_hash FROM audit_receipts WHERE subject_did = $1 ORDER BY created_at DESC LIMIT 1',
      [subscriberDid]
    );
    const prevHash = prevHashRow.rows[0]?.receipt_hash || 'GENESIS';
    const auditData = buildInactiveDeletionAuditMetadata({
      deletion_kind: 'subscriber_account',
      event_type: 'account_deleted',
      subject_did: subscriberDid,
      subscriber_id: subscriberId,
      email: subscriber.email,
      deleted_at: deletionTimestamp,
      records_deleted: parseInt(recordsCount.rows[0].count),
      credentials_deleted: parseInt(credentialsCount.rows[0].count),
      trustees_deleted: parseInt(trusteesCount.rows[0].count),
      consents_deleted: parseInt(consentsCount.rows[0].count),
    });
    const receiptHash = crypto
      .createHash('sha256')
      .update(JSON.stringify(auditData) + prevHash)
      .digest('hex');

    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash, previous_hash)
       VALUES ($1, $2, $3, $4, $5, $6, $7)`,
      [
        subscriberDid,
        subscriberDid,
        'account_deleted',
        'account_management',
        JSON.stringify(auditData),
        receiptHash,
        prevHash,
      ]
    );

    // Delete subscriber — ON DELETE CASCADE handles all associated data:
    // subscriber_allergies, subscriber_medications, subscriber_conditions, emergency_contacts,
    // trustees, cards, medical_records, record_requests, consent_events, credentials,
    // notifications (via recipient_did — NO FK, but subscriber row deleted),
    // odentity_scores, odentity_claims, device_signing_keys, research_opt_ins, vss_ceremonies
    await db.query('DELETE FROM subscribers WHERE id = $1', [subscriberId]);

    console.log(`[Subscribers] Account deleted: subscriber #${subscriberId} (${subscriber.email}), audit receipt: ${receiptHash.substring(0, 16)}...`);

    res.json({
      message: 'Account deleted successfully',
      deleted_at: deletionTimestamp,
      audit_receipt: receiptHash,
      cascade_deleted: {
        records: parseInt(recordsCount.rows[0].count),
        credentials: parseInt(credentialsCount.rows[0].count),
        trustees: parseInt(trusteesCount.rows[0].count),
        consents: parseInt(consentsCount.rows[0].count),
      },
    });
  } catch (err) {
    console.error('[Subscribers] Account deletion error:', err.message);
    res.status(500).json({ error: 'Failed to delete account' });
  }
});

// DELETE /api/subscribers/:id - Delete subscriber by ID (admin only)
router.delete('/:id', requireAnyAuth, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;

    const result = await db.query('DELETE FROM subscribers WHERE id = $1 RETURNING id', [parseInt(id)]);

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json({ message: 'Subscriber deleted', id: result.rows[0].id });
  } catch (err) {
    console.error('[Subscribers] Delete error:', err.message);
    res.status(500).json({ error: 'Failed to delete subscriber' });
  }
});

// POST /api/subscribers/phone/request - Request phone verification (sets phone + generates code)
router.post('/phone/request', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { phone } = req.body;

    if (!phone || typeof phone !== 'string' || phone.trim().length < 7) {
      return res.status(400).json({ error: 'Valid phone number is required' });
    }

    // Generate 6-digit verification code
    const code = Math.floor(100000 + Math.random() * 900000).toString();
    const expiresAt = new Date(Date.now() + 10 * 60 * 1000); // 10 minutes

    await db.query(
      `UPDATE subscribers SET
        phone = $1,
        phone_verified = FALSE,
        phone_verification_code = $2,
        phone_verification_expires_at = $3,
        updated_at = NOW()
      WHERE id = $4`,
      [phone.trim(), code, expiresAt, subscriberId]
    );

    // In dev mode, log the code to console (simulating SMS)
    console.log(`[SMS] Phone Verification Code for subscriber ${subscriberId}`);
    console.log(`  Phone: ${phone.trim()}`);
    console.log(`  Code: ${code}`);
    console.log(`  Expires: ${expiresAt.toISOString()}`);

    res.json(
      buildPublicPhoneVerificationRequestResponse({
        phone: phone.trim(),
        expiresAt: expiresAt.toISOString(),
        devCode: process.env.NODE_ENV === 'production' ? undefined : code,
      })
    );
  } catch (err) {
    console.error('[Subscribers] Phone request error:', err.message);
    res.status(500).json({ error: 'Failed to initiate phone verification' });
  }
});

// POST /api/subscribers/phone/confirm - Confirm phone verification code
router.post('/phone/confirm', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { code } = req.body;

    if (!code) {
      return res.status(400).json({ error: 'Verification code is required' });
    }

    // Get subscriber's current verification state
    const subResult = await db.query(
      'SELECT id, phone, phone_verified, phone_verification_code, phone_verification_expires_at FROM subscribers WHERE id = $1',
      [subscriberId]
    );

    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const sub = subResult.rows[0];

    if (sub.phone_verified) {
      return res.json(
        buildPublicPhoneVerificationConfirmResponse({
          phone: sub.phone,
          alreadyVerified: true,
        })
      );
    }

    if (!sub.phone_verification_code) {
      return res.status(400).json({ error: 'No verification pending. Please request a code first.' });
    }

    if (new Date() > new Date(sub.phone_verification_expires_at)) {
      return res.status(400).json({ error: 'Verification code expired. Please request a new code.' });
    }

    if (sub.phone_verification_code !== code.toString().trim()) {
      return res.status(400).json({ error: 'Invalid verification code' });
    }

    // Mark phone as verified
    await db.query(
      `UPDATE subscribers SET
        phone_verified = TRUE,
        phone_verification_code = NULL,
        phone_verification_expires_at = NULL,
        updated_at = NOW()
      WHERE id = $1`,
      [subscriberId]
    );

    // Award identity_core claim for phone verification (10 points) - only once
    const existingClaim = await db.query(
      "SELECT id FROM odentity_claims WHERE subscriber_id = $1 AND claim_type = 'phone_verified' AND dimension = 'identity_core'",
      [subscriberId]
    );

    let claimAwarded = false;
    if (existingClaim.rows.length === 0) {
      await db.query(
        "INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer) VALUES ($1, 'phone_verified', 'identity_core', 10, 'livesafe')",
        [subscriberId]
      );
      await db.query(
        `INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count)
         VALUES ($1, 'identity_core', 10, 1)
         ON CONFLICT (subscriber_id, dimension) DO UPDATE SET
           current_score = LEAST(odentity_scores.current_score + 10, odentity_scores.max_possible),
           claim_count = odentity_scores.claim_count + 1,
           last_updated = NOW()`,
        [subscriberId]
      );
      claimAwarded = true;
      console.log(`[0dentity] Awarded 10 identity_core points for phone verification to subscriber ${subscriberId}`);
    }

    res.json(
      buildPublicPhoneVerificationConfirmResponse({
        phone: sub.phone,
        identityCorePointsAwarded: claimAwarded ? 10 : 0,
      })
    );
  } catch (err) {
    console.error('[Subscribers] Phone confirm error:', err.message);
    res.status(500).json({ error: 'Failed to confirm phone verification' });
  }
});

module.exports = router;
