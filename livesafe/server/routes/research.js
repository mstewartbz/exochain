const express = require('express');
const router = express.Router();
const {
  buildResearchAuditTrailResponse,
  buildResearchOptInMutationResponse,
  buildResearchOptInResponse,
  buildResearchSubscriberTrialMatchResponse,
  buildResearchTrialConsentListResponse,
  buildResearchTrialConsentMutationResponse,
  buildResearchTrialConsentResponse,
} = require('../utils/research-response.js');

// Middleware to verify JWT
function authenticate(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  const token = authHeader.substring(7);
  try {
    const jwt = require('jsonwebtoken');
    const secret = process.env.JWT_SECRET;
    const decoded = jwt.verify(token, secret);
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

// ─── GET current research opt-in status ───────────────────────
// GET /api/research/opt-in
router.get('/opt-in', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    const result = await db.query(
      'SELECT * FROM research_opt_ins WHERE subscriber_id = $1',
      [subscriberId]
    );

    if (result.rows.length === 0) {
      return res.json(buildResearchOptInResponse(null));
    }

    res.json(buildResearchOptInResponse(result.rows[0]));
  } catch (err) {
    console.error('[Research] Get opt-in error:', err.message);
    res.status(500).json({ error: 'Failed to get research opt-in status' });
  }
});

// ─── POST opt into clinical trial matching ───────────────────
// POST /api/research/opt-in
router.post('/opt-in', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;

    // Generate a CyberMedica consent reference (simulated integration)
    const consentRef = 'CM-' + Date.now() + '-' + Math.random().toString(36).substring(2, 8).toUpperCase();
    const now = new Date();

    // Upsert the opt-in record
    const result = await db.query(
      `INSERT INTO research_opt_ins (subscriber_id, subscriber_did, opted_in, opt_in_at, opt_out_at, consent_scope, cybermedica_consent_ref, updated_at)
       VALUES ($1, $2, TRUE, $3, NULL, 'de_identified_trial_matching', $4, $3)
       ON CONFLICT (subscriber_id) DO UPDATE SET
         opted_in = TRUE,
         opt_in_at = $3,
         opt_out_at = NULL,
         cybermedica_consent_ref = $4,
         updated_at = $3
       RETURNING *`,
      [subscriberId, subscriberDid, now, consentRef]
    );

    // Record on audit trail
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $1, 'research_opt_in', 'de_identified_trial_matching', $2, $3)`,
      [
        subscriberDid,
        JSON.stringify({
          consent_scope: 'de_identified_trial_matching',
          cybermedica_consent_ref: consentRef,
          opt_in_at: now.toISOString(),
          policy: 'CyberMedica_Bridge_v1'
        }),
        require('crypto').randomUUID()
      ]
    );

    console.log('[Research] Subscriber', subscriberDid, 'opted in to clinical trial matching. ConsentRef:', consentRef);

    res.json(
      buildResearchOptInMutationResponse({
        record: result.rows[0],
        message: 'Successfully opted into clinical trial matching',
      })
    );
  } catch (err) {
    console.error('[Research] Opt-in error:', err.message);
    res.status(500).json({ error: 'Failed to opt into research bridge' });
  }
});

// ─── DELETE opt out of clinical trial matching ────────────────
// DELETE /api/research/opt-in
router.delete('/opt-in', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;

    const now = new Date();

    const result = await db.query(
      `UPDATE research_opt_ins SET opted_in = FALSE, opt_out_at = $1, updated_at = $1
       WHERE subscriber_id = $2 RETURNING *`,
      [now, subscriberId]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'No opt-in record found' });
    }

    // Record opt-out on audit trail
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $1, 'research_opt_out', 'de_identified_trial_matching', $2, $3)`,
      [
        subscriberDid,
        JSON.stringify({
          consent_scope: 'de_identified_trial_matching',
          opt_out_at: now.toISOString(),
          policy: 'CyberMedica_Bridge_v1'
        }),
        require('crypto').randomUUID()
      ]
    );

    console.log('[Research] Subscriber', subscriberDid, 'opted out of clinical trial matching');

    res.json(
      buildResearchOptInMutationResponse({
        record: result.rows[0],
        message: 'Successfully opted out of clinical trial matching',
      })
    );
  } catch (err) {
    console.error('[Research] Opt-out error:', err.message);
    res.status(500).json({ error: 'Failed to opt out of research bridge' });
  }
});

// ─── GET audit trail for research events ─────────────────────
// GET /api/research/audit
router.get('/audit', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberDid = req.user.did;

    const result = await db.query(
      `SELECT id, event_type, scope, details, created_at
       FROM audit_receipts
       WHERE subject_did = $1 AND event_type IN ('research_opt_in', 'research_opt_out', 'trial_eligibility_check', 'trial_consent_granted', 'trial_consent_withdrawn')
       ORDER BY created_at DESC`,
      [subscriberDid]
    );

    res.json(buildResearchAuditTrailResponse(result.rows));
  } catch (err) {
    console.error('[Research] Audit error:', err.message);
    res.status(500).json({ error: 'Failed to get research audit trail' });
  }
});

// ─── Clinical trial catalog (simulated CyberMedica trials) ───
const CLINICAL_TRIALS = [
  {
    id: 'CM-TRIAL-2026-001',
    title: 'Type 2 Diabetes Management with GLP-1 Agonist',
    sponsor: 'CyberMedica Research Institute',
    phase: 'Phase III',
    eligibility_criteria: {
      conditions: ['diabetes', 'type 2 diabetes', 'diabetes mellitus'],
      min_age: 18,
      max_age: 75,
      exclude_conditions: ['type 1 diabetes', 'severe renal failure'],
      exclude_medications: []
    },
    description: 'Evaluating the efficacy of next-generation GLP-1 agonist in managing HbA1c levels in T2DM patients.',
  },
  {
    id: 'CM-TRIAL-2026-002',
    title: 'Hypertension Control Study — Novel ACE Inhibitor',
    sponsor: 'CyberMedica Cardiovascular Division',
    phase: 'Phase II',
    eligibility_criteria: {
      conditions: ['hypertension', 'high blood pressure', 'hypertensive'],
      min_age: 21,
      max_age: 80,
      exclude_conditions: ['renal failure', 'severe heart failure'],
      exclude_medications: ['warfarin']
    },
    description: 'Investigating a novel ACE inhibitor for improved blood pressure control with reduced side effects.',
  },
  {
    id: 'CM-TRIAL-2026-003',
    title: 'Asthma Prevention via Biologic Therapy',
    sponsor: 'CyberMedica Respiratory Research',
    phase: 'Phase III',
    eligibility_criteria: {
      conditions: ['asthma', 'bronchial asthma', 'allergic asthma'],
      min_age: 12,
      max_age: 65,
      exclude_conditions: ['COPD', 'chronic obstructive pulmonary disease'],
      exclude_medications: []
    },
    description: 'Testing a biologic therapy to reduce asthma attack frequency and severity in moderate-to-severe cases.',
  },
  {
    id: 'CM-TRIAL-2026-004',
    title: 'General Wellness & Preventive Health Cohort',
    sponsor: 'CyberMedica Population Health',
    phase: 'Observational',
    eligibility_criteria: {
      conditions: [],
      min_age: 18,
      max_age: 99,
      exclude_conditions: [],
      exclude_medications: []
    },
    description: 'An observational cohort study tracking general health markers in de-identified volunteers across demographics.',
  },
];

// ─── ZK-proof eligibility check ────────────────────────────────
// This simulates generating a ZK proof that subscriber meets trial criteria
// WITHOUT exposing actual PHI to the research platform.
// The "proof" is a hash commitment to the eligibility result.
function generateZkProof(subscriberDid, trialId, eligible, timestamp) {
  const crypto = require('crypto');
  // ZK proof simulation: commit to (did, trialId, eligible, timestamp)
  // without revealing health data — in production this would use groth16
  const commitment = crypto.createHash('sha256')
    .update(subscriberDid + ':' + trialId + ':' + String(eligible) + ':' + timestamp)
    .digest('hex');
  return 'ZKP-' + commitment.substring(0, 32).toUpperCase();
}

// ─── GET matched clinical trials (ZK-protected eligibility check) ──
// GET /api/research/trials
// Returns matched trials with ZK proofs — subscriber PHI is NEVER included in response
router.get('/trials', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;

    // Verify subscriber is opted in
    const optInResult = await db.query(
      'SELECT opted_in FROM research_opt_ins WHERE subscriber_id = $1',
      [subscriberId]
    );

    if (optInResult.rows.length === 0 || !optInResult.rows[0].opted_in) {
      return res.status(403).json({
        error: 'Subscriber has not opted into clinical trial matching',
        code: 'NOT_OPTED_IN'
      });
    }

    // Fetch subscriber's health data (kept server-side for ZK proof computation)
    const [conditionsResult, subscriber] = await Promise.all([
      db.query('SELECT condition_name FROM subscriber_conditions WHERE subscriber_id = $1', [subscriberId]),
      db.query('SELECT date_of_birth, blood_type FROM subscribers WHERE id = $1', [subscriberId])
    ]);

    const subscriberConditions = conditionsResult.rows.map(r => r.condition_name.toLowerCase());
    const subscriberInfo = subscriber.rows[0] || {};

    // Calculate subscriber age (if DOB available)
    let subscriberAge = null;
    if (subscriberInfo.date_of_birth) {
      const dob = new Date(subscriberInfo.date_of_birth);
      const now = new Date();
      subscriberAge = Math.floor((now - dob) / (365.25 * 24 * 3600 * 1000));
    }

    const timestamp = new Date().toISOString();
    const matchedTrials = [];

    // Evaluate eligibility for each trial using ZK-style proof computation
    // PHI is evaluated SERVER-SIDE only — research platform sees only the proof
    for (const trial of CLINICAL_TRIALS) {
      const criteria = trial.eligibility_criteria;

      let eligible = true;
      const eligibilityFactors = [];

      // Check required conditions (if any)
      if (criteria.conditions.length > 0) {
        const hasRequiredCondition = criteria.conditions.some(reqCond =>
          subscriberConditions.some(sub => sub.includes(reqCond.toLowerCase()) || reqCond.toLowerCase().includes(sub))
        );
        if (!hasRequiredCondition) {
          eligible = false;
          eligibilityFactors.push('condition_criteria_not_met');
        }
      } else {
        eligibilityFactors.push('no_condition_requirement');
      }

      // Check exclusion criteria
      if (eligible && criteria.exclude_conditions.length > 0) {
        const hasExclusionCondition = criteria.exclude_conditions.some(excCond =>
          subscriberConditions.some(sub => sub.includes(excCond.toLowerCase()))
        );
        if (hasExclusionCondition) {
          eligible = false;
          eligibilityFactors.push('exclusion_criteria_met');
        }
      }

      // Check age range
      if (eligible && subscriberAge !== null) {
        if (subscriberAge < criteria.min_age || subscriberAge > criteria.max_age) {
          eligible = false;
          eligibilityFactors.push('age_out_of_range');
        }
      }

      // Generate ZK proof for this eligibility result
      // NOTE: zkProofRef proves eligibility without revealing WHY (no PHI leaked)
      const zkProofRef = generateZkProof(subscriberDid, trial.id, eligible, timestamp);

      matchedTrials.push({
        trial_id: trial.id,
        title: trial.title,
        sponsor: trial.sponsor,
        phase: trial.phase,
        description: trial.description,
        eligible: eligible,
        zk_proof_ref: zkProofRef,
        zk_scheme: 'groth16-simulated',
        checked_at: timestamp,
        // IMPORTANT: NO PHI returned — no conditions, no diagnoses, no personal data
        // The research platform receives only: eligible (true/false) + ZK proof reference
      });
    }

    // Log eligibility check in audit trail (no PHI in log)
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $1, 'trial_eligibility_check', 'de_identified_trial_matching', $2, $3)`,
      [
        subscriberDid,
        JSON.stringify({
          trials_checked: CLINICAL_TRIALS.length,
          eligible_count: matchedTrials.filter(t => t.eligible).length,
          checked_at: timestamp,
          phi_exposed: false,
          zk_scheme: 'groth16-simulated',
          policy: 'CyberMedica_Bridge_v1'
        }),
        require('crypto').randomUUID()
      ]
    );

    console.log('[Research] ZK eligibility check for', subscriberDid,
      '- eligible for', matchedTrials.filter(t => t.eligible).length, 'of', CLINICAL_TRIALS.length, 'trials');

    res.json(
      buildResearchSubscriberTrialMatchResponse({
        matchedTrials,
        totalChecked: CLINICAL_TRIALS.length,
        totalEligible: matchedTrials.filter(t => t.eligible).length,
        checkedAt: timestamp,
      })
    );
  } catch (err) {
    console.error('[Research] Trial matching error:', err.message);
    res.status(500).json({ error: 'Failed to check trial eligibility' });
  }
});

// ─── ZK Proof Storage Table ────────────────────────────────────
async function ensureZkProofsTable(db) {
  await db.query(`
    CREATE TABLE IF NOT EXISTS zk_proofs (
      id SERIAL PRIMARY KEY,
      proof_ref TEXT UNIQUE NOT NULL,
      subject_did TEXT NOT NULL,
      trial_id TEXT NOT NULL,
      eligible BOOLEAN NOT NULL,
      zk_scheme TEXT NOT NULL DEFAULT 'groth16-simulated',
      issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      verified_count INTEGER DEFAULT 0,
      phi_exposed BOOLEAN DEFAULT FALSE
    )
  `);
  await db.query('CREATE INDEX IF NOT EXISTS idx_zk_proofs_ref ON zk_proofs(proof_ref)');
}

// ─── POST /api/research/provider/eligibility ──────────────────
// Provider requests ZK eligibility verification for a subscriber
// Returns ONLY: eligible (bool), zk_proof_ref — NO PHI
router.post('/provider/eligibility', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;

    if (req.user.user_type !== 'provider' && req.user.role !== 'provider') {
      return res.status(403).json({ error: 'Only providers can request eligibility verification', code: 'NOT_PROVIDER' });
    }

    const providerId = req.user.id;
    const providerDid = req.user.did;
    const { subscriber_did, trial_id } = req.body;

    if (!subscriber_did || !trial_id) {
      return res.status(400).json({ error: 'subscriber_did and trial_id are required' });
    }

    // Find the trial
    const trial = CLINICAL_TRIALS.find(t => t.id === trial_id);
    if (!trial) {
      return res.status(404).json({ error: 'Trial not found', trial_id });
    }

    // Find the subscriber
    const subscriberResult = await db.query(
      'SELECT id, did, date_of_birth FROM subscribers WHERE did = $1',
      [subscriber_did]
    );
    if (subscriberResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subscriberResult.rows[0];
    const subscriberId = subscriber.id;

    // Verify subscriber has opted into research bridge (consent for de-identified trial matching)
    // Note: The ZK proof system uses the subscriber's research opt-in, NOT provider-level consent.
    // This allows CyberMedica research platforms to verify eligibility for opted-in subscribers.
    const optInResult = await db.query(
      'SELECT opted_in FROM research_opt_ins WHERE subscriber_id = $1',
      [subscriberId]
    );
    if (optInResult.rows.length === 0 || !optInResult.rows[0].opted_in) {
      return res.status(403).json({
        error: 'Subscriber has not opted into clinical trial matching',
        code: 'NOT_OPTED_IN'
      });
    }

    // Check if subscriber has specifically withdrawn from this trial
    // Feature #237: Trial withdrawal cleans up consent and access
    await ensureTrialConsentsTable(db);
    const trialConsentResult = await db.query(
      'SELECT status, withdrawn_at FROM trial_consents WHERE subscriber_id = $1 AND trial_id = $2',
      [subscriberId, trial_id]
    );
    if (trialConsentResult.rows.length > 0 && trialConsentResult.rows[0].status === 'withdrawn') {
      // Record the blocked access attempt in audit trail
      await db.query(
        `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
         VALUES ($1, $2, 'provider_access_blocked_withdrawal', 'clinical_trial_participation', $3, $4)`,
        [
          subscriber_did,
          providerDid,
          JSON.stringify({
            trial_id,
            reason: 'subscriber_withdrew_trial_consent',
            withdrawn_at: trialConsentResult.rows[0].withdrawn_at,
            provider_did: providerDid,
            blocked_at: new Date().toISOString(),
          }),
          require('crypto').randomUUID(),
        ]
      );
      return res.status(403).json({
        error: 'Subscriber has withdrawn from this clinical trial',
        code: 'TRIAL_CONSENT_WITHDRAWN',
        trial_id,
        message: 'Access denied: subscriber withdrew trial-specific consent'
      });
    }

    // Fetch subscriber's health data SERVER-SIDE only (ZK evaluation, no PHI in response)
    const conditionsResult = await db.query(
      'SELECT condition_name FROM subscriber_conditions WHERE subscriber_id = $1',
      [subscriberId]
    );

    const subscriberConditions = conditionsResult.rows.map(r => r.condition_name.toLowerCase());

    // Calculate age
    let subscriberAge = null;
    if (subscriber.date_of_birth) {
      const dob = new Date(subscriber.date_of_birth);
      const now = new Date();
      subscriberAge = Math.floor((now - dob) / (365.25 * 24 * 3600 * 1000));
    }

    const criteria = trial.eligibility_criteria;
    const timestamp = new Date().toISOString();
    let eligible = true;

    // Evaluate eligibility SERVER-SIDE
    if (criteria.conditions.length > 0) {
      const hasRequired = criteria.conditions.some(reqCond =>
        subscriberConditions.some(sub => sub.includes(reqCond.toLowerCase()) || reqCond.toLowerCase().includes(sub))
      );
      if (!hasRequired) eligible = false;
    }
    if (eligible && criteria.exclude_conditions.length > 0) {
      const hasExclusion = criteria.exclude_conditions.some(excCond =>
        subscriberConditions.some(sub => sub.includes(excCond.toLowerCase()))
      );
      if (hasExclusion) eligible = false;
    }
    if (eligible && subscriberAge !== null) {
      if (subscriberAge < criteria.min_age || subscriberAge > criteria.max_age) eligible = false;
    }

    // Generate ZK proof (hash commitment — no PHI included)
    const zkProofRef = generateZkProof(subscriber_did, trial_id, eligible, timestamp);

    // Store proof for independent verification
    await ensureZkProofsTable(db);
    await db.query(
      `INSERT INTO zk_proofs (proof_ref, subject_did, trial_id, eligible, issued_at)
       VALUES ($1, $2, $3, $4, $5)
       ON CONFLICT (proof_ref) DO UPDATE SET verified_count = zk_proofs.verified_count + 1`,
      [zkProofRef, subscriber_did, trial_id, eligible, timestamp]
    );

    // Audit log — no PHI, records provider DID and proof ref only
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $2, 'provider_eligibility_check', 'zk_proof', $3, $4)`,
      [
        subscriber_did,
        providerDid,
        JSON.stringify({
          trial_id,
          zk_proof_ref: zkProofRef,
          eligible,
          phi_exposed: false,
          zk_scheme: 'groth16-simulated',
          provider_did: providerDid,
          checked_at: timestamp,
        }),
        require('crypto').randomUUID(),
      ]
    );

    console.log('[Research] Provider', providerDid, 'eligibility check for', subscriber_did,
      '— trial:', trial_id, '— eligible:', eligible, '— ZK proof:', zkProofRef);

    // Response: ONLY eligibility + proof reference. NO PHI.
    res.json({
      eligible,
      zk_proof_ref: zkProofRef,
      trial_id,
      trial_title: trial.title,
      zk_scheme: 'groth16-simulated',
      phi_exposed: false,
      checked_at: timestamp,
      message: eligible
        ? 'Subscriber meets eligibility criteria. ZK proof generated.'
        : 'Subscriber does not meet eligibility criteria.',
    });
  } catch (err) {
    console.error('[Research] Provider eligibility error:', err.message);
    res.status(500).json({ error: 'Eligibility check failed' });
  }
});

// ─── GET /api/research/verify-proof/:proofRef ─────────────────
// Independently verify a ZK proof reference
// No authentication required — proof is publicly verifiable
router.get('/verify-proof/:proofRef', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { proofRef } = req.params;

    await ensureZkProofsTable(db);

    const result = await db.query(
      `UPDATE zk_proofs SET verified_count = verified_count + 1
       WHERE proof_ref = $1
       RETURNING proof_ref, trial_id, eligible, zk_scheme, issued_at, verified_count, phi_exposed`,
      [proofRef]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({
        valid: false,
        error: 'Proof reference not found',
        proof_ref: proofRef,
      });
    }

    const proof = result.rows[0];

    res.json({
      valid: true,
      proof_ref: proof.proof_ref,
      trial_id: proof.trial_id,
      eligible: proof.eligible,
      zk_scheme: proof.zk_scheme,
      issued_at: proof.issued_at,
      verified_count: proof.verified_count,
      phi_exposed: proof.phi_exposed,
      // Note: subject DID is NOT returned (privacy protection — independently verifiable)
    });
  } catch (err) {
    console.error('[Research] Verify proof error:', err.message);
    res.status(500).json({ error: 'Proof verification failed' });
  }
});

// ─── Trial Consent Table Setup ────────────────────────────────
// Creates trial_consents table if it doesn't exist
async function ensureTrialConsentsTable(db) {
  await db.query(`
    CREATE TABLE IF NOT EXISTS trial_consents (
      id SERIAL PRIMARY KEY,
      subscriber_id INTEGER NOT NULL REFERENCES subscribers(id),
      subscriber_did TEXT NOT NULL,
      trial_id TEXT NOT NULL,
      trial_title TEXT,
      zk_proof_ref TEXT,
      consented_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      withdrawn_at TIMESTAMPTZ,
      consent_ref TEXT NOT NULL,
      status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'withdrawn')),
      UNIQUE(subscriber_id, trial_id)
    )
  `);
}

// ─── GET /api/research/trial-consents - List subscriber's trial consents ──
router.get('/trial-consents', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    await ensureTrialConsentsTable(db);
    const subscriberId = req.user.id;

    const result = await db.query(
      'SELECT * FROM trial_consents WHERE subscriber_id = $1 ORDER BY consented_at DESC',
      [subscriberId]
    );

    res.json(buildResearchTrialConsentListResponse(result.rows));
  } catch (err) {
    console.error('[Research] Get trial consents error:', err.message);
    res.status(500).json({ error: 'Failed to get trial consents' });
  }
});

// ─── POST /api/research/trials/:trialId/consent - Grant consent for specific trial ──
// Feature #151: Subscriber grants trial-specific consent
router.post('/trials/:trialId/consent', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    await ensureTrialConsentsTable(db);

    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;
    const { trialId } = req.params;

    // Verify subscriber is opted in
    const optInResult = await db.query(
      'SELECT opted_in FROM research_opt_ins WHERE subscriber_id = $1',
      [subscriberId]
    );
    if (optInResult.rows.length === 0 || !optInResult.rows[0].opted_in) {
      return res.status(403).json({
        error: 'Subscriber has not opted into clinical trial matching',
        code: 'NOT_OPTED_IN'
      });
    }

    // Find trial details
    const trial = CLINICAL_TRIALS.find(t => t.id === trialId);
    if (!trial) {
      return res.status(404).json({ error: 'Trial not found', trial_id: trialId });
    }

    // Check if already consented (active)
    const existing = await db.query(
      'SELECT * FROM trial_consents WHERE subscriber_id = $1 AND trial_id = $2',
      [subscriberId, trialId]
    );
    if (existing.rows.length > 0 && existing.rows[0].status === 'active') {
      return res.status(409).json({
        error: 'Already enrolled in this trial',
        consent: buildResearchTrialConsentResponse(existing.rows[0]),
      });
    }

    // Generate consent reference
    const crypto = require('crypto');
    const consentRef = 'TC-' + Date.now() + '-' + crypto.randomBytes(4).toString('hex').toUpperCase();
    const timestamp = new Date().toISOString();

    // Generate ZK proof reference for this consent
    const zkProofRef = generateZkProof(subscriberDid, trialId, true, timestamp);

    let consentRecord;
    if (existing.rows.length > 0) {
      // Re-enroll after withdrawal
      const result = await db.query(
        `UPDATE trial_consents SET status = 'active', consented_at = $1, withdrawn_at = NULL,
         consent_ref = $2, zk_proof_ref = $3
         WHERE subscriber_id = $4 AND trial_id = $5 RETURNING *`,
        [timestamp, consentRef, zkProofRef, subscriberId, trialId]
      );
      consentRecord = result.rows[0];
    } else {
      // New consent
      const result = await db.query(
        `INSERT INTO trial_consents (subscriber_id, subscriber_did, trial_id, trial_title, zk_proof_ref, consent_ref, status, consented_at)
         VALUES ($1, $2, $3, $4, $5, $6, 'active', $7)
         RETURNING *`,
        [subscriberId, subscriberDid, trialId, trial.title, zkProofRef, consentRef, timestamp]
      );
      consentRecord = result.rows[0];
    }

    // Record audit event referencing subscriber DID (Feature #151 requirement)
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $1, 'trial_consent_granted', 'clinical_trial_participation', $2, $3)`,
      [
        subscriberDid,
        JSON.stringify({
          trial_id: trialId,
          trial_title: trial.title,
          consent_ref: consentRef,
          zk_proof_ref: zkProofRef,
          subscriber_did: subscriberDid,
          consented_at: timestamp,
          policy: 'CyberMedica_Bridge_v1'
        }),
        require('crypto').randomUUID()
      ]
    );

    console.log('[Research] Subscriber', subscriberDid, 'granted consent for trial', trialId, '- ref:', consentRef);

    res.status(201).json(
      buildResearchTrialConsentMutationResponse({
        consent: consentRecord,
        message: 'Successfully enrolled in trial: ' + trial.title,
        trialId,
        trialTitle: trial.title,
        status: 'enrolled',
        changedAtKey: 'consented_at',
        changedAt: timestamp,
      })
    );
  } catch (err) {
    console.error('[Research] Grant trial consent error:', err.message);
    res.status(500).json({ error: 'Failed to grant trial consent' });
  }
});

// ─── DELETE /api/research/trials/:trialId/consent - Withdraw from trial ──
// Feature #152: Subscriber can withdraw from trial
router.delete('/trials/:trialId/consent', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    await ensureTrialConsentsTable(db);

    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;
    const { trialId } = req.params;

    // Find existing active consent
    const existing = await db.query(
      'SELECT * FROM trial_consents WHERE subscriber_id = $1 AND trial_id = $2 AND status = $3',
      [subscriberId, trialId, 'active']
    );

    if (existing.rows.length === 0) {
      return res.status(404).json({
        error: 'No active enrollment found for this trial',
        trial_id: trialId
      });
    }

    const consentRecord = existing.rows[0];
    const withdrawnAt = new Date().toISOString();

    // Mark as withdrawn
    await db.query(
      'UPDATE trial_consents SET status = $1, withdrawn_at = $2 WHERE id = $3',
      ['withdrawn', withdrawnAt, consentRecord.id]
    );

    // Record withdrawal in audit trail (Feature #152 requirement)
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $1, 'trial_consent_withdrawn', 'clinical_trial_participation', $2, $3)`,
      [
        subscriberDid,
        JSON.stringify({
          trial_id: trialId,
          trial_title: consentRecord.trial_title,
          consent_ref: consentRecord.consent_ref,
          original_consent_at: consentRecord.consented_at,
          withdrawn_at: withdrawnAt,
          subscriber_did: subscriberDid,
          policy: 'CyberMedica_Bridge_v1'
        }),
        require('crypto').randomUUID()
      ]
    );

    console.log('[Research] Subscriber', subscriberDid, 'withdrew from trial', trialId);

    res.json(
      buildResearchTrialConsentMutationResponse({
        consent: consentRecord,
        message: 'Successfully withdrawn from trial: ' + (consentRecord.trial_title || trialId),
        trialId,
        trialTitle: consentRecord.trial_title || trialId,
        status: 'withdrawn',
        changedAtKey: 'withdrawn_at',
        changedAt: withdrawnAt,
      })
    );
  } catch (err) {
    console.error('[Research] Withdraw trial consent error:', err.message);
    res.status(500).json({ error: 'Failed to withdraw from trial' });
  }
});

module.exports = router;
