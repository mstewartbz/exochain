const express = require('express');
const router = express.Router();
const bcrypt = require('bcryptjs');
const jwt = require('jsonwebtoken');
const { v4: uuidv4 } = require('uuid');
const { sendError } = require('../utils/errorHandler');
const { authMiddleware } = require('../middleware/auth');
const { runtimeExochainAdapter } = require('../utils/livesafe-exochain-adapter');
const { buildTrusteeVssStatusSummary } = require('../utils/trustee-vss-summary');
const {
  buildPublicSubscriberAuthResponse,
  buildPublicSubscriberAuthSessionResponse,
} = require('../utils/auth-subscriber-response');
const {
  buildPublicResponderAuthResponse,
  buildPublicResponderAuthSessionResponse,
  buildPublicAgencyDirectoryEntry,
  buildPublicAgencyRegistrationResponse,
  buildPublicAgencyRegistrationSessionResponse,
} = require('../utils/auth-responder-response');
const {
  buildPublicProviderAuthResponse,
  buildPublicProviderAuthSessionResponse,
  buildPublicProviderAuthProfileResponse,
  buildPublicProviderNpiLookupResponse,
} = require('../utils/auth-provider-response');
const {
  buildPublicTrusteeAuthResponse,
  buildPublicTrusteeAuthSessionResponse,
  buildPublicTrusteeProfileResponse,
} = require('../utils/auth-trustee-response');
const {
  buildPublicEmailVerificationResponse,
} = require('../utils/verification-response');

const JWT_SECRET = process.env.JWT_SECRET;
const VIEW_AS_ROLES = new Set(['subscriber']);
const VIEW_AS_SESSION_EXPIRES_IN = '30m';

// Email validation helper
function isValidEmail(email) {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

// POST /api/auth/register - Register a new subscriber
router.post('/register', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password, first_name, last_name, is_hero, is_military } = req.body;

    // Server-side validation: collect field-level errors
    const errors = {};

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

    if (Object.keys(errors).length > 0) {
      return res.status(400).json({ error: 'Validation failed', errors });
    }

    const did = `did:exo:subscriber:${uuidv4()}`;
    const password_hash = await bcrypt.hash(password, 10);
    const verification_token = uuidv4();
    const heroFlag =
      is_hero === true ||
      is_hero === 'true' ||
      is_military === true ||
      is_military === 'true';

    const result = await db.query(
      `INSERT INTO subscribers (did, email, password_hash, first_name, last_name, verification_token, is_hero, is_military)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
       RETURNING id, did, email, first_name, last_name, role, email_verified, is_hero, is_military, created_at`,
      [did, email, password_hash, first_name || null, last_name || null, verification_token, heroFlag, heroFlag]
    );

    // Log verification link to console (dev mode email)
    console.log(`[EMAIL] Verification link for ${email}: http://localhost:3000/verify?token=${verification_token}`);
    if (heroFlag) {
      console.log(`[ACCOUNT] Heroes free tier account registered: ${email}`);
    }

    const token = jwt.sign({ id: result.rows[0].id, did, role: 'subscriber' }, JWT_SECRET, { expiresIn: '24h' });

    // EXOCHAIN Phase 2: anchor to immutable ledger
    runtimeExochainAdapter.registerIdentity(did).then(anchor => {
      if (anchor) console.log(`[EXOCHAIN] Identity anchor confirmed for ${did}`);
    }).catch(err => {
      console.warn(`[EXOCHAIN] Identity anchor failed (non-fatal): ${err.message}`);
    });

    res.status(201).json(
      buildPublicSubscriberAuthSessionResponse({
        user: result.rows[0],
        token,
      })
    );
  } catch (err) {
    if (err.code === '23505') {
      return res.status(409).json({ error: 'Email already exists' });
    }
    console.error('[Auth] Register error:', err.message);
    sendError(res, err, 'Registration failed. Please try again.');
  }
});

// POST /api/auth/login - Login
router.post('/login', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password } = req.body;

    if (!email || !password) {
      return res.status(400).json({ error: 'Email and password are required' });
    }

    const result = await db.query('SELECT * FROM subscribers WHERE email = $1', [email]);

    if (result.rows.length === 0) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    const user = result.rows[0];
    const validPassword = await bcrypt.compare(password, user.password_hash);

    if (!validPassword) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    // Admin accounts get 8-hour sessions; regular subscribers get 24-hour sessions
    const tokenExpiry = user.role === 'subscriber_admin' ? '8h' : '24h';
    const token = jwt.sign({ id: user.id, did: user.did, role: user.role }, JWT_SECRET, { expiresIn: tokenExpiry });

    res.json(
      buildPublicSubscriberAuthSessionResponse({
        user,
        token,
        sessionExpiresIn: tokenExpiry,
      })
    );
  } catch (err) {
    console.error('[Auth] Login error:', err.message);
    sendError(res, err, 'Login failed. Please try again.');
  }
});

// POST /api/auth/view-as - Issue a short-lived effective-role session for admins
router.post('/view-as', authMiddleware, async (req, res) => {
  try {
    const { role } = req.body || {};
    if (!VIEW_AS_ROLES.has(role)) {
      return res.status(400).json({ error: 'Unsupported view-as role' });
    }
    if (req.user.view_as_mode) {
      return res.status(403).json({ error: 'Exit view-as mode before starting a new view-as session' });
    }

    const db = req.app.locals.db;
    const result = await db.query(
      'SELECT id, did, email, first_name, last_name, role, email_verified, is_hero, is_military FROM subscribers WHERE id = $1',
      [req.user.id]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'User not found' });
    }

    const admin = result.rows[0];
    if (admin.role !== 'subscriber_admin' || req.user.role !== 'subscriber_admin') {
      return res.status(403).json({ error: 'Admin access required' });
    }

    const token = jwt.sign(
      {
        id: admin.id,
        did: admin.did,
        role,
        actual_role: admin.role,
        view_as_role: role,
        view_as_mode: true,
        user_type: 'subscriber',
      },
      JWT_SECRET,
      { expiresIn: VIEW_AS_SESSION_EXPIRES_IN }
    );
    const decoded = jwt.decode(token);
    const viewAsUser = {
      ...admin,
      role,
      view_as: true,
      actual_role: admin.role,
      view_as_role: role,
      view_as_expires_at: decoded?.exp ? new Date(decoded.exp * 1000).toISOString() : null,
    };

    res.json(
      buildPublicSubscriberAuthSessionResponse({
        user: viewAsUser,
        token,
        sessionExpiresIn: VIEW_AS_SESSION_EXPIRES_IN,
      })
    );
  } catch (err) {
    console.error('[Auth] View-as error:', err.message);
    res.status(500).json({ error: 'Failed to start view-as session' });
  }
});

// 405 handler for non-POST methods on /api/auth/login (Feature #258)
router.all('/login', (req, res) => {
  res.setHeader('Allow', 'POST');
  res.status(405).json({ error: 'Method Not Allowed', allowed_methods: ['POST'] });
});

// 405 handler for non-POST methods on /api/auth/register (Feature #258)
router.all('/register', (req, res) => {
  res.setHeader('Allow', 'POST');
  res.status(405).json({ error: 'Method Not Allowed', allowed_methods: ['POST'] });
});

// GET /api/auth/verify-email - Verify email with token
router.get('/verify-email', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { token } = req.query;

    if (!token) {
      return res.status(400).json({ error: 'Verification token is required' });
    }

    // Look up subscriber by verification token
    const result = await db.query(
      'SELECT id, did, email, email_verified, verification_token FROM subscribers WHERE verification_token = $1',
      [token]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Invalid or expired verification token' });
    }

    const subscriber = result.rows[0];

    if (subscriber.email_verified) {
      return res.json(
        buildPublicEmailVerificationResponse({
          email: subscriber.email,
          alreadyVerified: true,
        })
      );
    }

    // Mark email as verified (keep token for idempotent re-verification)
    await db.query(
      'UPDATE subscribers SET email_verified = TRUE, updated_at = NOW() WHERE id = $1',
      [subscriber.id]
    );

    // Award identity_core claim for email verification (10 points)
    const existingClaim = await db.query(
      "SELECT id FROM odentity_claims WHERE subscriber_id = $1 AND claim_type = 'email_verified' AND dimension = 'identity_core'",
      [subscriber.id]
    );
    if (existingClaim.rows.length === 0) {
      await db.query(
        "INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer) VALUES ($1, 'email_verified', 'identity_core', 10, 'livesafe')",
        [subscriber.id]
      );
      await db.query(
        "INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count) VALUES ($1, 'identity_core', 10, 1) ON CONFLICT (subscriber_id, dimension) DO UPDATE SET current_score = LEAST(odentity_scores.current_score + 10, odentity_scores.max_possible), claim_count = odentity_scores.claim_count + 1, last_updated = NOW()",
        [subscriber.id]
      );
      console.log(`[0dentity] Awarded 10 identity_core points for email verification to subscriber ${subscriber.id}`);
    }

    console.log(`[EMAIL] Email verified for ${subscriber.email} (DID: ${subscriber.did})`);

    res.json(
      buildPublicEmailVerificationResponse({
        email: subscriber.email,
      })
    );
  } catch (err) {
    console.error('[Auth] Verify email error:', err.message);
    res.status(500).json({ error: 'Email verification failed' });
  }
});

// GET /api/auth/me - Get current user
router.get('/me', async (req, res) => {
  try {
    const authHeader = req.headers.authorization;
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return res.status(401).json({ error: 'No token provided' });
    }

    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);
    const db = req.app.locals.db;

    const result = await db.query(
      'SELECT id, did, email, first_name, last_name, role, email_verified, is_hero, is_military FROM subscribers WHERE id = $1',
      [decoded.id]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'User not found' });
    }

    const user = result.rows[0];
    if (decoded.view_as_mode) {
      if (
        user.role !== 'subscriber_admin' ||
        decoded.actual_role !== 'subscriber_admin' ||
        !VIEW_AS_ROLES.has(decoded.view_as_role) ||
        decoded.role !== decoded.view_as_role
      ) {
        return res.status(403).json({ error: 'Invalid view-as session' });
      }

      return res.json(buildPublicSubscriberAuthResponse({
        ...user,
        role: decoded.view_as_role,
        view_as: true,
        actual_role: user.role,
        view_as_role: decoded.view_as_role,
        view_as_expires_at: decoded.exp ? new Date(decoded.exp * 1000).toISOString() : null,
      }));
    }

    res.json(buildPublicSubscriberAuthResponse(user));
  } catch (err) {
    if (err.name === 'JsonWebTokenError' || err.name === 'TokenExpiredError') {
      return res.status(401).json({ error: 'Invalid or expired token' });
    }
    console.error('[Auth] Me error:', err.message);
    res.status(500).json({ error: 'Failed to get user' });
  }
});

// =============================================================================
// TRUSTEE AUTH ROUTES
// =============================================================================

// POST /api/auth/trustee/login - Login as trustee
router.post('/trustee/login', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password } = req.body;

    if (!email || !password) {
      return res.status(400).json({ error: 'Email and password are required' });
    }

    // Find an accepted trustee with this email (any subscriber)
    const result = await db.query(
      `SELECT id, did, email, password_hash, role, first_name, last_name
       FROM trustees
       WHERE LOWER(email) = LOWER($1) AND status = 'accepted' AND did IS NOT NULL
       ORDER BY accepted_at ASC LIMIT 1`,
      [email]
    );

    if (result.rows.length === 0) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    const trustee = result.rows[0];
    const validPassword = await bcrypt.compare(password, trustee.password_hash);

    if (!validPassword) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    const token = jwt.sign(
      { id: trustee.id, did: trustee.did, role: 'trustee', user_type: 'trustee', email: trustee.email },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    res.json(
      buildPublicTrusteeAuthSessionResponse({
        user: trustee,
        token,
      })
    );
  } catch (err) {
    console.error('[Auth] Trustee login error:', err.message);
    res.status(500).json({ error: 'Login failed' });
  }
});

// GET /api/auth/trustee/me - Get current trustee profile with all trusteeships
router.get('/trustee/me', async (req, res) => {
  try {
    const authHeader = req.headers.authorization;
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return res.status(401).json({ error: 'No token provided' });
    }

    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);

    if (decoded.user_type !== 'trustee') {
      return res.status(403).json({ error: 'Not a trustee account' });
    }

    const db = req.app.locals.db;

    // Get all trusteeships for this trustee email
    const result = await db.query(
      `SELECT t.id, t.did, t.email, t.role, t.status, t.shard_ref, t.accepted_at, t.first_name, t.last_name,
              s.first_name as subscriber_first_name, s.last_name as subscriber_last_name, s.email as subscriber_email,
              s.did as subscriber_did, s.email_verified as subscriber_email_verified, s.id as subscriber_id
       FROM trustees t
       JOIN subscribers s ON t.subscriber_id = s.id
       WHERE LOWER(t.email) = LOWER($1) AND t.status = 'accepted'
       ORDER BY t.accepted_at`,
      [decoded.email]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Trustee not found' });
    }

    // For each subscriber, count their accepted trustees to determine PACE completion status
    const subscriberIds = [...new Set(result.rows.map(r => r.subscriber_id))];
    const paceCountResult = await db.query(
      `SELECT subscriber_id, COUNT(*) as accepted_count
       FROM trustees
       WHERE subscriber_id = ANY($1) AND status = 'accepted'
       GROUP BY subscriber_id`,
      [subscriberIds]
    );
    const paceCountMap = {};
    paceCountResult.rows.forEach(r => {
      paceCountMap[r.subscriber_id] = parseInt(r.accepted_count, 10);
    });

    const first = result.rows[0];
    res.json(
      buildPublicTrusteeProfileResponse({
        trustee: buildPublicTrusteeAuthResponse(first),
        trusteeships: result.rows.map(r => {
          const vssSummary = buildTrusteeVssStatusSummary(r);
          const paceCount = paceCountMap[r.subscriber_id] || 0;
          const subscriberStatus = r.subscriber_email_verified
            ? (paceCount >= 4 ? 'protected' : 'active')
            : 'unverified';

          return {
            id: r.id,
            role: r.role,
            ...vssSummary,
            accepted_at: r.accepted_at,
            subscriber_name: [r.subscriber_first_name, r.subscriber_last_name].filter(Boolean).join(' ') || r.subscriber_email,
            subscriber_status: subscriberStatus,
            subscriber_pace_count: paceCount,
          };
        }),
      })
    );
  } catch (err) {
    if (err.name === 'JsonWebTokenError' || err.name === 'TokenExpiredError') {
      return res.status(401).json({ error: 'Invalid or expired token' });
    }
    console.error('[Auth] Trustee me error:', err.message);
    res.status(500).json({ error: 'Failed to get trustee profile' });
  }
});

// =============================================================================
// RESPONDER AUTH ROUTES
// =============================================================================

// POST /api/auth/responder/register - Register a first responder
router.post('/responder/register', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password, agency_name, agency_type, role, certification, is_military } = req.body;

    if (!email || !password) {
      return res.status(400).json({ error: 'Email and password are required' });
    }

    if (!agency_name || !agency_type) {
      return res.status(400).json({ error: 'Agency name and type are required' });
    }

    if (!role) {
      return res.status(400).json({ error: 'Role/position is required' });
    }

    // Check if agency already exists by name and type
    let agency;
    const existingAgency = await db.query(
      'SELECT * FROM agencies WHERE LOWER(name) = LOWER($1) AND LOWER(type) = LOWER($2)',
      [agency_name.trim(), agency_type.trim()]
    );

    if (existingAgency.rows.length > 0) {
      agency = existingAgency.rows[0];
    } else {
      // Create the agency
      const agencyResult = await db.query(
        'INSERT INTO agencies (name, type, admin_email) VALUES ($1, $2, $3) RETURNING *',
        [agency_name.trim(), agency_type.trim(), email]
      );
      agency = agencyResult.rows[0];
    }

    // Create responder account
    const did = `did:exo:responder:${uuidv4()}`;
    const password_hash = await bcrypt.hash(password, 10);

    const result = await db.query(
      `INSERT INTO responders (did, email, password_hash, agency_id, role, certification, is_military)
       VALUES ($1, $2, $3, $4, $5, $6, $7)
       RETURNING id, did, email, agency_id, role, certification, is_military, created_at`,
      [did, email, password_hash, agency.id, role, certification || null, is_military || false]
    );

    const responder = result.rows[0];

    const token = jwt.sign(
      { id: responder.id, did, role: 'responder', user_type: 'responder' },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    console.log(`[AUTH] Responder registered: ${email} (DID: ${did}) - Agency: ${agency.name} (${agency.type})`);

    res.status(201).json(
      buildPublicResponderAuthSessionResponse({
        user: {
          id: responder.id,
          did: responder.did,
          email: responder.email,
          role: responder.role,
          certification: responder.certification,
          is_military: responder.is_military,
          agency_name: agency.name,
          agency_type: agency.type,
        },
        token,
      })
    );
  } catch (err) {
    if (err.code === '23505') {
      return res.status(409).json({ error: 'Email already registered' });
    }
    console.error('[Auth] Responder register error:', err.message);
    res.status(500).json({ error: 'Registration failed' });
  }
});

// POST /api/auth/responder/login - Login as responder
router.post('/responder/login', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password } = req.body;

    if (!email || !password) {
      return res.status(400).json({ error: 'Email and password are required' });
    }

    const result = await db.query(
      `SELECT r.*, a.name as agency_name, a.type as agency_type
       FROM responders r
       LEFT JOIN agencies a ON r.agency_id = a.id
       WHERE r.email = $1`,
      [email]
    );

    if (result.rows.length === 0) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    const responder = result.rows[0];
    const validPassword = await bcrypt.compare(password, responder.password_hash);

    if (!validPassword) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    const token = jwt.sign(
      { id: responder.id, did: responder.did, role: 'responder', user_type: 'responder' },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    res.json(
      buildPublicResponderAuthSessionResponse({
        user: responder,
        token,
      })
    );
  } catch (err) {
    console.error('[Auth] Responder login error:', err.message);
    res.status(500).json({ error: 'Login failed' });
  }
});

// GET /api/auth/responder/me - Get current responder profile
router.get('/responder/me', async (req, res) => {
  try {
    const authHeader = req.headers.authorization;
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return res.status(401).json({ error: 'No token provided' });
    }

    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);

    if (decoded.user_type !== 'responder') {
      return res.status(403).json({ error: 'Not a responder account' });
    }

    const db = req.app.locals.db;
    const result = await db.query(
      `SELECT r.id, r.did, r.email, r.agency_id, r.role, r.certification, r.is_military, r.created_at,
              a.name as agency_name, a.type as agency_type
       FROM responders r
       LEFT JOIN agencies a ON r.agency_id = a.id
       WHERE r.id = $1`,
      [decoded.id]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Responder not found' });
    }

    const responder = result.rows[0];
    res.json(buildPublicResponderAuthResponse(responder));
  } catch (err) {
    if (err.name === 'JsonWebTokenError' || err.name === 'TokenExpiredError') {
      return res.status(401).json({ error: 'Invalid or expired token' });
    }
    console.error('[Auth] Responder me error:', err.message);
    res.status(500).json({ error: 'Failed to get responder profile' });
  }
});

// =============================================================================
// AGENCY ROUTES
// =============================================================================

// POST /api/auth/agency/register - Register a new agency
router.post('/agency/register', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { name, type, admin_email, admin_password, credentials } = req.body;

    if (!name || !type || !admin_email || !admin_password) {
      return res.status(400).json({ error: 'Agency name, type, admin email, and admin password are required' });
    }

    const validTypes = ['fire', 'ems', 'hospital', 'police', 'military'];
    if (!validTypes.includes(type.toLowerCase())) {
      return res.status(400).json({ error: `Agency type must be one of: ${validTypes.join(', ')}` });
    }

    // Check if agency already exists
    const existingAgency = await db.query(
      'SELECT * FROM agencies WHERE LOWER(name) = LOWER($1) AND LOWER(type) = LOWER($2)',
      [name.trim(), type.trim()]
    );

    if (existingAgency.rows.length > 0) {
      return res.status(409).json({ error: 'Agency with this name and type already exists' });
    }

    // Create agency
    const agencyResult = await db.query(
      'INSERT INTO agencies (name, type, admin_email, verified) VALUES ($1, $2, $3, $4) RETURNING *',
      [name.trim(), type.trim().toLowerCase(), admin_email, false]
    );
    const agency = agencyResult.rows[0];

    // Create admin as a responder account
    const did = `did:exo:responder:${uuidv4()}`;
    const password_hash = await bcrypt.hash(admin_password, 10);

    const responderResult = await db.query(
      `INSERT INTO responders (did, email, password_hash, agency_id, role, certification)
       VALUES ($1, $2, $3, $4, $5, $6)
       RETURNING id, did, email, agency_id, role, certification, is_military, created_at`,
      [did, admin_email, password_hash, agency.id, 'agency_admin', credentials || null]
    );

    const admin = responderResult.rows[0];

    const token = jwt.sign(
      { id: admin.id, did, role: 'responder', user_type: 'responder' },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    console.log(`[AUTH] Agency registered: ${agency.name} (${agency.type}) - Admin: ${admin_email}`);

    res.status(201).json(
      buildPublicAgencyRegistrationSessionResponse({
        agency,
        admin: {
          id: admin.id,
          did: admin.did,
          email: admin.email,
          role: admin.role,
          certification: admin.certification,
          is_military: admin.is_military,
          agency_name: agency.name,
          agency_type: agency.type,
        },
        token,
      })
    );
  } catch (err) {
    if (err.code === '23505') {
      return res.status(409).json({ error: 'Admin email already registered' });
    }
    console.error('[Auth] Agency register error:', err.message);
    res.status(500).json({ error: 'Agency registration failed' });
  }
});

// GET /api/auth/agencies - List all agencies (for registration dropdown)
router.get('/agencies', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query('SELECT id, name, type, verified, created_at FROM agencies ORDER BY name');
    res.json(result.rows.map(buildPublicAgencyDirectoryEntry));
  } catch (err) {
    console.error('[Auth] List agencies error:', err.message);
    res.status(500).json({ error: 'Failed to list agencies' });
  }
});

// =============================================================================
// PROVIDER AUTH ROUTES
// =============================================================================

// NPI Luhn check algorithm (real NPI validation)
function validateNpiLuhn(npi) {
  if (!npi || npi.length !== 10) return false;
  // Prefix with 80840 for Luhn check (CMS standard)
  const prefixed = '80840' + npi;
  let sum = 0;
  let alternate = false;
  for (let i = prefixed.length - 1; i >= 0; i--) {
    let n = parseInt(prefixed[i], 10);
    if (alternate) {
      n *= 2;
      if (n > 9) n -= 9;
    }
    sum += n;
    alternate = !alternate;
  }
  return sum % 10 === 0;
}

// Simulated NPI Registry lookup (in production, this calls NPPES API)
function lookupNpiRegistry(npi) {
  // Validate format and Luhn check
  if (!validateNpiLuhn(npi)) {
    return null;
  }

  // Generate deterministic provider info from NPI for dev mode
  // In production, this would call https://npiregistry.cms.hhs.gov/api/
  var firstDigit = parseInt(npi[0], 10);
  var secondDigit = parseInt(npi[1], 10);

  var firstNames = ['James', 'Sarah', 'Michael', 'Emily', 'Robert', 'Jennifer', 'William', 'Lisa', 'David', 'Maria'];
  var lastNames = ['Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis', 'Rodriguez', 'Martinez'];
  var taxonomies = [
    'Internal Medicine', 'Family Medicine', 'Emergency Medicine',
    'Cardiology', 'Neurology', 'Orthopedics',
    'Pediatrics', 'Surgery', 'Radiology', 'Psychiatry'
  ];
  var facilities = [
    'Memorial General Hospital', 'St. Joseph Medical Center', 'University Health System',
    'Regional Medical Center', 'City General Hospital', 'Valley Health Partners',
    'Summit Healthcare', 'Mercy Medical Center', 'Riverside Hospital', 'Pine Valley Clinic'
  ];

  var firstName = firstNames[firstDigit];
  var lastName = lastNames[secondDigit];
  var taxonomy = taxonomies[(firstDigit + secondDigit) % 10];
  var facility = facilities[(firstDigit * 2 + secondDigit) % 10];

  return {
    npi: npi,
    provider_name: firstName + ' ' + lastName,
    first_name: firstName,
    last_name: lastName,
    taxonomy_description: taxonomy,
    facility: facility,
    status: 'Active',
    enumeration_type: 'NPI-1',
    enumeration_date: '2015-03-15',
    last_updated: '2024-01-10',
    addresses: [
      {
        address_purpose: 'LOCATION',
        city: 'Springfield',
        state: 'IL',
        postal_code: '62701'
      }
    ]
  };
}

// GET /api/auth/provider/npi-lookup/:npi - Look up NPI in registry
router.get('/provider/npi-lookup/:npi', async (req, res) => {
  try {
    var npi = req.params.npi.replace(/\D/g, '');

    if (npi.length !== 10) {
      return res.status(400).json({ error: 'NPI must be a 10-digit number' });
    }

    // Check if NPI is already registered in our system
    var db = req.app.locals.db;
    var existing = await db.query('SELECT id, did, email FROM providers WHERE npi = $1', [npi]);
    if (existing.rows.length > 0) {
      return res.status(409).json({
        error: 'This NPI is already registered in the system',
        already_registered: true
      });
    }

    // Look up NPI in registry
    var result = lookupNpiRegistry(npi);

    if (!result) {
      return res.status(404).json({
        error: 'NPI not found in the National Provider Registry. Please verify your NPI number.',
        valid: false
      });
    }

    console.log('[NPI] Lookup successful for NPI: ' + npi + ' - Provider: ' + result.provider_name);

    res.json(buildPublicProviderNpiLookupResponse(result));
  } catch (err) {
    console.error('[Auth] NPI lookup error:', err.message);
    res.status(500).json({ error: 'NPI lookup failed' });
  }
});

// POST /api/auth/provider/register - Register a healthcare provider
router.post('/provider/register', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password, npi, facility, specialty, provider_name, npi_taxonomy } = req.body;

    if (!email || !password) {
      return res.status(400).json({ error: 'Email and password are required' });
    }

    if (!npi) {
      return res.status(400).json({ error: 'NPI number is required' });
    }

    // Validate NPI format (10 digits)
    const npiClean = npi.replace(/\D/g, '');
    if (npiClean.length !== 10) {
      return res.status(400).json({ error: 'NPI must be a 10-digit number' });
    }

    if (!facility) {
      return res.status(400).json({ error: 'Facility affiliation is required' });
    }

    // Verify NPI using Luhn check
    const npiValid = validateNpiLuhn(npiClean);

    // Check if NPI already registered
    const existingNpi = await db.query('SELECT id FROM providers WHERE npi = $1', [npiClean]);
    if (existingNpi.rows.length > 0) {
      return res.status(409).json({ error: 'NPI already registered' });
    }

    // Look up NPI in registry for verification
    const npiInfo = lookupNpiRegistry(npiClean);
    const isNpiVerified = npiInfo !== null;
    const resolvedProviderName = provider_name || (npiInfo ? npiInfo.provider_name : null);
    const resolvedTaxonomy = npi_taxonomy || (npiInfo ? npiInfo.taxonomy_description : null);

    // Create provider account
    const did = `did:exo:provider:${uuidv4()}`;
    const password_hash = await bcrypt.hash(password, 10);

    const result = await db.query(
      `INSERT INTO providers (did, email, password_hash, npi, facility, specialty, verified, npi_verified, provider_name, npi_taxonomy)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
       RETURNING id, did, email, npi, facility, specialty, verified, npi_verified, provider_name, npi_taxonomy, created_at`,
      [did, email, password_hash, npiClean, facility.trim(), specialty || resolvedTaxonomy || null, false, isNpiVerified, resolvedProviderName, resolvedTaxonomy]
    );

    const provider = result.rows[0];

    const token = jwt.sign(
      { id: provider.id, did, role: 'provider', user_type: 'provider' },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    console.log(`[AUTH] Provider registered: ${email} (DID: ${did}) - NPI: ${npiClean} (verified: ${isNpiVerified}) - Facility: ${facility}`);

    res.status(201).json(
      buildPublicProviderAuthSessionResponse({
        user: provider,
        token,
      })
    );
  } catch (err) {
    if (err.code === '23505') {
      return res.status(409).json({ error: 'Email or NPI already registered' });
    }
    console.error('[Auth] Provider register error:', err.message);
    res.status(500).json({ error: 'Registration failed' });
  }
});

// POST /api/auth/provider/login - Login as provider
router.post('/provider/login', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { email, password } = req.body;

    if (!email || !password) {
      return res.status(400).json({ error: 'Email and password are required' });
    }

    const result = await db.query('SELECT * FROM providers WHERE email = $1', [email]);

    if (result.rows.length === 0) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    const provider = result.rows[0];
    const validPassword = await bcrypt.compare(password, provider.password_hash);

    if (!validPassword) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    const token = jwt.sign(
      { id: provider.id, did: provider.did, role: 'provider', user_type: 'provider' },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    res.json(
      buildPublicProviderAuthSessionResponse({
        user: provider,
        token,
      })
    );
  } catch (err) {
    console.error('[Auth] Provider login error:', err.message);
    res.status(500).json({ error: 'Login failed' });
  }
});

// GET /api/auth/provider/me - Get current provider profile
router.get('/provider/me', async (req, res) => {
  try {
    const authHeader = req.headers.authorization;
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return res.status(401).json({ error: 'No token provided' });
    }

    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);

    if (decoded.user_type !== 'provider') {
      return res.status(403).json({ error: 'Not a provider account' });
    }

    const db = req.app.locals.db;
    const result = await db.query(
      'SELECT id, did, email, npi, facility, specialty, verified, npi_verified, provider_name, npi_taxonomy FROM providers WHERE id = $1',
      [decoded.id]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Provider not found' });
    }

    const provider = result.rows[0];

    // Fetch consents granted to this provider
    const consentsResult = await db.query(
      `SELECT ce.id, ce.subscriber_id, ce.scope, ce.purpose, ce.granted_at, ce.expires_at, ce.revoked_at,
              s.first_name as subscriber_first_name, s.last_name as subscriber_last_name
       FROM consent_events ce
       JOIN subscribers s ON ce.subscriber_id = s.id
       WHERE ce.provider_id = $1
       ORDER BY ce.granted_at DESC`,
      [decoded.id]
    );

    const consents = consentsResult.rows.map(c => ({
      id: c.id,
      subscriber_name: [c.subscriber_first_name, c.subscriber_last_name].filter(Boolean).join(' ') || 'Anonymous',
      scope: c.scope,
      purpose: c.purpose,
      created_at: c.granted_at,
      expires_at: c.expires_at,
      revoked_at: c.revoked_at,
    }));

    res.json(
      buildPublicProviderAuthProfileResponse({
        provider: buildPublicProviderAuthResponse(provider),
        consents,
      })
    );
  } catch (err) {
    if (err.name === 'JsonWebTokenError' || err.name === 'TokenExpiredError') {
      return res.status(401).json({ error: 'Invalid or expired token' });
    }
    console.error('[Auth] Provider me error:', err.message);
    res.status(500).json({ error: 'Failed to get provider profile' });
  }
});

module.exports = router;
