const express = require('express');
const router = express.Router();
const { authMiddleware } = require('../middleware/auth');
const { buildAuditImmutabilityError } = require('../utils/audit-immutability-policy');
const {
  buildAdminAgencyResponderListResponse,
  buildAdminResponderListResponse,
  buildAdminResponderResponse,
  buildAdminResponderToggleResponse,
} = require('../utils/admin-responder-response');
const {
  buildAdminAgencyListResponse,
  buildAdminAgencyMutationResponse,
} = require('../utils/admin-agency-response');
const {
  buildAdminStatsResponse,
} = require('../utils/admin-stats-response');
const {
  buildAdminSubscriberListResponse,
  buildAdminSubscriberResponse,
} = require('../utils/admin-subscriber-response');
const {
  buildAdminAuditTrailResponse,
} = require('../utils/admin-audit-response');

// Admin-only middleware
function adminOnly(req, res, next) {
  if (!req.user || req.user.role !== 'subscriber_admin') {
    return res.status(403).json({ error: 'Admin access required' });
  }
  next();
}

async function fetchAdminAgencySummary(db, id) {
  const result = await db.query(
    `SELECT a.id, a.name, a.type, a.is_active, a.created_at,
            COUNT(r.id) AS responder_count,
            COUNT(CASE WHEN r.is_active THEN 1 END) AS active_responders
     FROM agencies a
     LEFT JOIN responders r ON r.agency_id = a.id
     WHERE a.id = $1
     GROUP BY a.id`,
    [id]
  );

  return result.rows[0] || null;
}

// GET /api/admin/stats - Platform statistics
router.get('/stats', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;

    const [subResult, providerResult, recordResult, scanResult] = await Promise.all([
      db.query('SELECT COUNT(*) AS total, COUNT(CASE WHEN role = \'subscriber_admin\' THEN 1 END) AS admins FROM subscribers'),
      db.query('SELECT COUNT(*) AS total FROM providers'),
      db.query('SELECT COUNT(*) AS total FROM medical_records'),
      db.query('SELECT COUNT(*) AS total FROM scans'),
    ]);

    res.json(buildAdminStatsResponse({
      subscribers: {
        total: subResult.rows[0].total,
        admins: subResult.rows[0].admins,
      },
      providers: providerResult.rows[0].total,
      medical_records: recordResult.rows[0].total,
      scans: scanResult.rows[0].total,
    }));
  } catch (err) {
    console.error('[Admin] Stats error:', err.message);
    res.status(500).json({ error: 'Failed to fetch stats' });
  }
});

// GET /api/admin/subscribers - List all subscribers (account management)
router.get('/subscribers', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { page = 1, limit = 20, search = '' } = req.query;
    const offset = (parseInt(page) - 1) * parseInt(limit);

    let query = `
      SELECT id, email, first_name, last_name, role, email_verified, created_at, updated_at
      FROM subscribers
    `;
    const params = [];

    if (search) {
      query += ` WHERE (email ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1)`;
      params.push(`%${search}%`);
      query += ` ORDER BY created_at DESC LIMIT $${params.length + 1} OFFSET $${params.length + 2}`;
      params.push(parseInt(limit), offset);
    } else {
      query += ` ORDER BY created_at DESC LIMIT $1 OFFSET $2`;
      params.push(parseInt(limit), offset);
    }

    const result = await db.query(query, params);

    // Count total
    const countQuery = search
      ? `SELECT COUNT(*) FROM subscribers WHERE (email ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1)`
      : `SELECT COUNT(*) FROM subscribers`;
    const countResult = await db.query(countQuery, search ? [`%${search}%`] : []);

    res.json(buildAdminSubscriberListResponse(result.rows, {
      total: parseInt(countResult.rows[0].count),
      page: parseInt(page),
      limit: parseInt(limit),
    }));
  } catch (err) {
    console.error('[Admin] List subscribers error:', err.message);
    res.status(500).json({ error: 'Failed to fetch subscribers' });
  }
});

// GET /api/admin/subscribers/:id - Get single subscriber details
router.get('/subscribers/:id', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;

    const result = await db.query(
      `SELECT id, email, first_name, last_name, role, email_verified, created_at, updated_at
       FROM subscribers WHERE id = $1`,
      [id]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    res.json(buildAdminSubscriberResponse(result.rows[0]));
  } catch (err) {
    console.error('[Admin] Get subscriber error:', err.message);
    res.status(500).json({ error: 'Failed to fetch subscriber' });
  }
});

// PATCH /api/admin/subscribers/:id - Update subscriber account (role, email_verified)
router.patch('/subscribers/:id', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;
    const { role, email_verified } = req.body;

    // Validate role if provided
    const allowedRoles = ['subscriber', 'subscriber_admin'];
    if (role && !allowedRoles.includes(role)) {
      return res.status(400).json({ error: `Invalid role. Must be one of: ${allowedRoles.join(', ')}` });
    }

    // Build update query dynamically
    const updates = [];
    const params = [];

    if (role !== undefined) {
      params.push(role);
      updates.push(`role = $${params.length}`);
    }

    if (email_verified !== undefined) {
      params.push(email_verified);
      updates.push(`email_verified = $${params.length}`);
    }

    if (updates.length === 0) {
      return res.status(400).json({ error: 'No fields to update' });
    }

    params.push(id);
    const query = `
      UPDATE subscribers SET ${updates.join(', ')}, updated_at = NOW()
      WHERE id = $${params.length}
      RETURNING id, email, first_name, last_name, role, email_verified, created_at, updated_at
    `;

    const result = await db.query(query, params);

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    console.log(`[Admin] Subscriber ${id} updated by admin ${req.user.id}`);
    res.json(buildAdminSubscriberResponse(result.rows[0]));
  } catch (err) {
    console.error('[Admin] Update subscriber error:', err.message);
    res.status(500).json({ error: 'Failed to update subscriber' });
  }
});

// ── AGENCY MANAGEMENT ──────────────────────────────────────────────────────

// GET /api/admin/agencies - List all agencies
router.get('/agencies', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      `SELECT a.id, a.name, a.type, a.is_active, a.created_at,
              COUNT(r.id) AS responder_count,
              COUNT(CASE WHEN r.is_active THEN 1 END) AS active_responders
       FROM agencies a
       LEFT JOIN responders r ON r.agency_id = a.id
       GROUP BY a.id
       ORDER BY a.created_at DESC`
    );
    res.json(buildAdminAgencyListResponse(result.rows));
  } catch (err) {
    console.error('[Admin] List agencies error:', err.message);
    res.status(500).json({ error: 'Failed to fetch agencies' });
  }
});

// DELETE /api/admin/agencies/:id - Deactivate an agency (cascade to responders)
router.delete('/agencies/:id', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;

    // Verify agency exists
    const agencyResult = await db.query(
      'SELECT id, name, type, is_active, created_at FROM agencies WHERE id = $1',
      [id]
    );
    if (agencyResult.rows.length === 0) {
      return res.status(404).json({ error: 'Agency not found' });
    }
    const agency = agencyResult.rows[0];

    if (!agency.is_active) {
      return res.status(400).json({ error: 'Agency is already deactivated' });
    }

    // Deactivate all responders first (cascade)
    const responderResult = await db.query(
      `UPDATE responders SET is_active = FALSE WHERE agency_id = $1 AND is_active = TRUE RETURNING id`,
      [id]
    );

    // Deactivate the agency
    await db.query('UPDATE agencies SET is_active = FALSE WHERE id = $1', [id]);

    const deactivatedCount = responderResult.rows.length;
    const agencySummary = await fetchAdminAgencySummary(db, id);
    console.log(`[Admin] Agency ${id} (${agency.name}) deactivated by admin ${req.user.id}. ${deactivatedCount} responder(s) deactivated.`);

    res.json(buildAdminAgencyMutationResponse({
      message: `Agency "${agency.name}" deactivated successfully`,
      agency: agencySummary,
      affected_responders: deactivatedCount,
    }));
  } catch (err) {
    console.error('[Admin] Deactivate agency error:', err.message);
    res.status(500).json({ error: 'Failed to deactivate agency' });
  }
});

// POST /api/admin/agencies/:id/reactivate - Reactivate a deactivated agency
router.post('/agencies/:id/reactivate', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;

    const agencyResult = await db.query(
      'SELECT id, name, type, is_active, created_at FROM agencies WHERE id = $1',
      [id]
    );
    if (agencyResult.rows.length === 0) {
      return res.status(404).json({ error: 'Agency not found' });
    }
    const agency = agencyResult.rows[0];

    // Reactivate agency and its responders
    await db.query('UPDATE agencies SET is_active = TRUE WHERE id = $1', [id]);
    const responderResult = await db.query(
      `UPDATE responders SET is_active = TRUE WHERE agency_id = $1 RETURNING id`,
      [id]
    );
    const agencySummary = await fetchAdminAgencySummary(db, id);

    res.json(buildAdminAgencyMutationResponse({
      message: `Agency "${agency.name}" reactivated`,
      agency: agencySummary,
      affected_responders: responderResult.rows.length,
    }));
  } catch (err) {
    console.error('[Admin] Reactivate agency error:', err.message);
    res.status(500).json({ error: 'Failed to reactivate agency' });
  }
});

// GET /api/admin/agencies/:id/responders - List responders for a specific agency
router.get('/agencies/:id/responders', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;

    const agencyResult = await db.query('SELECT * FROM agencies WHERE id = $1', [id]);
    if (agencyResult.rows.length === 0) {
      return res.status(404).json({ error: 'Agency not found' });
    }

    const result = await db.query(
      `SELECT id, email, role, certification, is_military, is_active, created_at
       FROM responders WHERE agency_id = $1 ORDER BY created_at ASC`,
      [id]
    );

    res.json(buildAdminAgencyResponderListResponse({
      agency: agencyResult.rows[0],
      responders: result.rows,
    }));
  } catch (err) {
    console.error('[Admin] List agency responders error:', err.message);
    res.status(500).json({ error: 'Failed to fetch agency responders' });
  }
});

// PATCH /api/admin/responders/:id - Toggle a responder's active status
router.patch('/responders/:id', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;
    const { is_active } = req.body;

    if (typeof is_active !== 'boolean') {
      return res.status(400).json({ error: 'is_active must be a boolean' });
    }

    const result = await db.query(
      'UPDATE responders SET is_active = $1 WHERE id = $2 RETURNING id, email, role, certification, is_military, is_active, created_at',
      [is_active, parseInt(id)]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Responder not found' });
    }

    const responder = result.rows[0];
    console.log(`[Admin] Responder ${id} (${responder.email}) ${is_active ? 'activated' : 'deactivated'} by admin ${req.user.id}`);

    res.json(buildAdminResponderToggleResponse({
      is_active,
      responder,
    }));
  } catch (err) {
    console.error('[Admin] Toggle responder status error:', err.message);
    res.status(500).json({ error: 'Failed to update responder status' });
  }
});

// ── AUDIT TRAIL PROTECTION ──────────────────────────────────
// Admin CANNOT modify audit records - all attempts are rejected

const AUDIT_IMMUTABILITY_ERROR = buildAuditImmutabilityError();

router.put('/audit/:id', authMiddleware, adminOnly, (req, res) => {
  console.log(`[Admin] Rejected PUT on audit record ${req.params.id} by admin ${req.user?.id}`);
  res.status(403).json(AUDIT_IMMUTABILITY_ERROR);
});

router.patch('/audit/:id', authMiddleware, adminOnly, (req, res) => {
  console.log(`[Admin] Rejected PATCH on audit record ${req.params.id} by admin ${req.user?.id}`);
  res.status(403).json(AUDIT_IMMUTABILITY_ERROR);
});

router.delete('/audit/:id', authMiddleware, adminOnly, (req, res) => {
  console.log(`[Admin] Rejected DELETE on audit record ${req.params.id} by admin ${req.user?.id}`);
  res.status(403).json(AUDIT_IMMUTABILITY_ERROR);
});

router.delete('/audit', authMiddleware, adminOnly, (req, res) => {
  console.log(`[Admin] Rejected bulk DELETE on audit by admin ${req.user?.id}`);
  res.status(403).json(AUDIT_IMMUTABILITY_ERROR);
});

// GET /api/admin/audit - Admins CAN read audit records
router.get('/audit', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { page = 1, limit = 20 } = req.query;
    const offset = (parseInt(page) - 1) * parseInt(limit);

    const result = await db.query(
      `SELECT id, actor_did, event_type, scope, details, receipt_hash, created_at
       FROM audit_receipts
       ORDER BY created_at DESC
       LIMIT $1 OFFSET $2`,
      [parseInt(limit), offset]
    );

    const countResult = await db.query('SELECT COUNT(*) FROM audit_receipts');

    res.json(buildAdminAuditTrailResponse(result.rows, {
      total: parseInt(countResult.rows[0].count),
      page: parseInt(page),
      limit: parseInt(limit),
    }));
  } catch (err) {
    console.error('[Admin] Get audit error:', err.message);
    res.status(500).json({ error: 'Failed to fetch audit records' });
  }
});

// GET /api/admin/marketplace/import-batches - Imported catalog provenance
router.get('/marketplace/import-batches', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      `SELECT id, source_label, zip_sha256, export_sha256, exported_at,
              entity_counts, import_mode, review_status, created_at
       FROM marketplace_import_batches
       ORDER BY created_at DESC
       LIMIT 50`
    );

    res.json({
      batches: result.rows.map((row) => ({
        id: row.id,
        source_label: row.source_label,
        zip_sha256: row.zip_sha256,
        export_sha256: row.export_sha256,
        exported_at: row.exported_at,
        entity_counts: row.entity_counts || {},
        import_mode: row.import_mode,
        review_status: row.review_status,
        created_at: row.created_at,
      })),
      total: result.rows.length,
    });
  } catch (err) {
    console.error('[Admin] Marketplace import batches error:', err.message);
    res.status(500).json({ error: 'Failed to fetch marketplace import batches' });
  }
});

// GET /api/admin/marketplace/quarantine - Quarantined imported records
router.get('/marketplace/quarantine', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const result = await db.query(
      `SELECT id, batch_id, source_entity, source_id, review_status,
              safe_excerpt, created_at
       FROM marketplace_import_records
       WHERE review_status = 'quarantined_sensitive'
       ORDER BY created_at DESC
       LIMIT 100`
    );

    res.json({
      quarantined_records: result.rows.map((row) => ({
        id: row.id,
        batch_id: row.batch_id,
        source_entity: row.source_entity,
        source_id: row.source_id,
        review_status: row.review_status,
        safe_excerpt: row.safe_excerpt || {},
        created_at: row.created_at,
      })),
      total: result.rows.length,
    });
  } catch (err) {
    console.error('[Admin] Marketplace quarantine error:', err.message);
    res.status(500).json({ error: 'Failed to fetch marketplace quarantine' });
  }
});

// PATCH /api/admin/marketplace/catalog/:id/review - Review imported catalog item
router.patch('/marketplace/catalog/:id/review', authMiddleware, adminOnly, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const id = Number.parseInt(String(req.params.id || ''), 10);
    const allowedStatuses = new Set(['draft', 'active', 'disabled', 'quarantined']);
    const allowedReview = new Set(['pending', 'reviewed', 'quarantined_sensitive', 'rejected']);
    const launchStatus = allowedStatuses.has(req.body.launch_status)
      ? req.body.launch_status
      : null;
    const reviewStatus = allowedReview.has(req.body.review_status)
      ? req.body.review_status
      : null;

    if (!Number.isFinite(id) || !launchStatus || !reviewStatus) {
      return res.status(400).json({
        error: 'Valid launch_status and review_status are required.',
        code: 'MARKETPLACE_REVIEW_INVALID',
      });
    }

    const result = await db.query(
      `UPDATE marketplace_catalog_items
       SET launch_status = $1,
           review_status = $2,
           updated_at = NOW()
       WHERE id = $3
       RETURNING id, slug, title, launch_status, review_status,
                 contains_sensitive_info, public_claims_allowed`,
      [launchStatus, reviewStatus, id]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Marketplace catalog item not found' });
    }

    res.json({
      item: result.rows[0],
      reviewed_by_admin_id: req.user.id,
    });
  } catch (err) {
    console.error('[Admin] Marketplace review error:', err.message);
    res.status(500).json({ error: 'Failed to review marketplace item' });
  }
});

module.exports = router;
