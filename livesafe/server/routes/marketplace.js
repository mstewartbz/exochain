const express = require('express');
const router = express.Router();
const { authMiddleware } = require('../middleware/auth');
const {
  buildMarketplaceCatalogItemResponse,
  buildMarketplaceCatalogListResponse,
  buildMarketplaceInstallResponse,
  buildMarketplaceLibraryResponse,
  buildMarketplaceReportResponse,
  buildMarketplaceRoleListResponse,
} = require('../utils/marketplace-response');

function featureDisabled(value) {
  return ['false', '0', 'off', 'disabled'].includes(String(value || '').toLowerCase());
}

function assertCatalogEnabled(res) {
  if (featureDisabled(process.env.LIVESAFE_MARKETPLACE_CATALOG_ENABLED)) {
    res.status(503).json({
      error: 'Marketplace catalog is disabled.',
      code: 'MARKETPLACE_CATALOG_DISABLED',
    });
    return false;
  }
  return true;
}

function assertInstallsEnabled(res) {
  if (featureDisabled(process.env.LIVESAFE_MARKETPLACE_INSTALLS_ENABLED)) {
    res.status(503).json({
      error: 'Marketplace installs are disabled.',
      code: 'MARKETPLACE_INSTALLS_DISABLED',
    });
    return false;
  }
  return true;
}

function normalizeLimit(value, fallback = 50, max = 100) {
  const parsed = Number.parseInt(String(value || ''), 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return fallback;
  }
  return Math.min(parsed, max);
}

function normalizeSearch(value) {
  const trimmed = String(value || '').trim();
  return trimmed.length > 120 ? trimmed.slice(0, 120) : trimmed;
}

function validateReportReason(value) {
  const allowed = new Set([
    'inappropriate_content',
    'spam',
    'copyright_violation',
    'malicious_code',
    'misleading_description',
    'other',
  ]);
  return allowed.has(value) ? value : null;
}

// GET /api/marketplace/catalog
router.get('/catalog', async (req, res) => {
  if (!assertCatalogEnabled(res)) return;

  try {
    const db = req.app.locals.db;
    const limit = normalizeLimit(req.query.limit);
    const search = normalizeSearch(req.query.search);
    const params = [];
    const where = [
      "visibility = 'public'",
      "launch_status = 'active'",
      "review_status = 'reviewed'",
      "contains_sensitive_info = FALSE",
      "public_claims_allowed = FALSE",
    ];

    if (req.query.category && req.query.category !== 'all') {
      params.push(String(req.query.category));
      where.push(`category = $${params.length}`);
    }

    if (req.query.object_type && req.query.object_type !== 'all') {
      params.push(String(req.query.object_type));
      where.push(`object_type = $${params.length}`);
    }

    if (search) {
      params.push(`%${search}%`);
      where.push(`(title ILIKE $${params.length} OR summary ILIKE $${params.length})`);
    }

    params.push(limit);
    const result = await db.query(
      `SELECT *
       FROM marketplace_catalog_items
       WHERE ${where.join(' AND ')}
       ORDER BY historical_install_count DESC, title ASC
       LIMIT $${params.length}`,
      params,
    );

    res.json(buildMarketplaceCatalogListResponse(result.rows));
  } catch (err) {
    console.error('[Marketplace] Catalog error:', err.message);
    res.status(500).json({ error: 'Failed to fetch marketplace catalog' });
  }
});

// GET /api/marketplace/catalog/:slug
router.get('/catalog/:slug', async (req, res) => {
  if (!assertCatalogEnabled(res)) return;

  try {
    const db = req.app.locals.db;
    const result = await db.query(
      `SELECT *
       FROM marketplace_catalog_items
       WHERE slug = $1
         AND visibility = 'public'
         AND launch_status = 'active'
         AND review_status = 'reviewed'
         AND contains_sensitive_info = FALSE
         AND public_claims_allowed = FALSE`,
      [req.params.slug],
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Marketplace item not found' });
    }

    res.json(buildMarketplaceCatalogItemResponse(result.rows[0]));
  } catch (err) {
    console.error('[Marketplace] Catalog item error:', err.message);
    res.status(500).json({ error: 'Failed to fetch marketplace item' });
  }
});

// GET /api/marketplace/roles
router.get('/roles', async (req, res) => {
  if (!assertCatalogEnabled(res)) return;

  try {
    const db = req.app.locals.db;
    const result = await db.query(
      `SELECT *
       FROM marketplace_agent_roles
       WHERE is_active = TRUE
         AND public_claims_allowed = FALSE
       ORDER BY role_name ASC`,
    );
    res.json(buildMarketplaceRoleListResponse(result.rows));
  } catch (err) {
    console.error('[Marketplace] Roles error:', err.message);
    res.status(500).json({ error: 'Failed to fetch marketplace roles' });
  }
});

// GET /api/marketplace/library
router.get('/library', authMiddleware, async (req, res) => {
  if (!assertInstallsEnabled(res)) return;

  try {
    const db = req.app.locals.db;
    const result = await db.query(
      `SELECT mui.id, mui.marketplace_item_id, mci.slug, mci.title, mui.installed_at
       FROM marketplace_user_installs mui
       JOIN marketplace_catalog_items mci ON mci.id = mui.marketplace_item_id
       WHERE mui.subscriber_id = $1
       ORDER BY mui.installed_at DESC`,
      [req.user.id],
    );
    res.json(buildMarketplaceLibraryResponse(result.rows));
  } catch (err) {
    console.error('[Marketplace] Library error:', err.message);
    res.status(500).json({ error: 'Failed to fetch marketplace library' });
  }
});

// POST /api/marketplace/installs
router.post('/installs', authMiddleware, async (req, res) => {
  if (!assertInstallsEnabled(res)) return;

  try {
    const db = req.app.locals.db;
    const marketplaceItemId = Number.parseInt(String(req.body.marketplace_item_id || ''), 10);
    if (!Number.isFinite(marketplaceItemId)) {
      return res.status(400).json({
        error: 'marketplace_item_id is required.',
        code: 'MARKETPLACE_ITEM_REQUIRED',
      });
    }

    const itemResult = await db.query(
      `SELECT id, slug, title
       FROM marketplace_catalog_items
       WHERE id = $1
         AND visibility = 'public'
         AND launch_status = 'active'
         AND review_status = 'reviewed'
         AND contains_sensitive_info = FALSE
         AND public_claims_allowed = FALSE`,
      [marketplaceItemId],
    );

    if (itemResult.rows.length === 0) {
      return res.status(404).json({ error: 'Marketplace item not found' });
    }

    const installResult = await db.query(
      `INSERT INTO marketplace_user_installs (
         subscriber_id, marketplace_item_id, install_type, source_version
       )
       VALUES ($1, $2, 'public_get', $3)
       ON CONFLICT (subscriber_id, marketplace_item_id)
       DO UPDATE SET updated_at = NOW()
       RETURNING id, marketplace_item_id, installed_at`,
      [req.user.id, marketplaceItemId, req.body.source_version || null],
    );

    const item = itemResult.rows[0];
    res.status(201).json(
      buildMarketplaceInstallResponse({
        ...installResult.rows[0],
        slug: item.slug,
        title: item.title,
      }),
    );
  } catch (err) {
    console.error('[Marketplace] Install error:', err.message);
    res.status(500).json({ error: 'Failed to install marketplace item' });
  }
});

// POST /api/marketplace/catalog/:id/report
router.post('/catalog/:id/report', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const marketplaceItemId = Number.parseInt(String(req.params.id || ''), 10);
    const reason = validateReportReason(req.body.report_reason);
    const details = String(req.body.report_details || '').trim().slice(0, 1000);

    if (!Number.isFinite(marketplaceItemId) || !reason) {
      return res.status(400).json({
        error: 'Valid marketplace item and report reason are required.',
        code: 'MARKETPLACE_REPORT_INVALID',
      });
    }

    const reportResult = await db.query(
      `INSERT INTO marketplace_reports (
         marketplace_item_id, reporter_subscriber_id, report_reason, report_details
       )
       VALUES ($1, $2, $3, $4)
       RETURNING id, marketplace_item_id, status`,
      [marketplaceItemId, req.user.id, reason, details || null],
    );

    res.status(201).json(buildMarketplaceReportResponse(reportResult.rows[0]));
  } catch (err) {
    console.error('[Marketplace] Report error:', err.message);
    res.status(500).json({ error: 'Failed to submit marketplace report' });
  }
});

module.exports = router;
