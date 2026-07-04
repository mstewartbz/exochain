const express = require('express');
const router = express.Router();
const PDFDocument = require('pdfkit');
const { JWT_SECRET, authMiddleware } = require('../middleware/auth');
const {
  buildAuditImmutabilityError,
  LOCAL_AUDIT_IMMUTABILITY_NOTE,
} = require('../utils/audit-immutability-policy');
const {
  buildAuditEventResponse,
  buildAuditTrailResponse,
} = require('../utils/audit-response');

const IMMUTABILITY_ERROR = buildAuditImmutabilityError();
const AUDIT_EVENT_SELECT =
  'SELECT id, actor_did, event_type, scope, details, receipt_hash, created_at FROM audit_receipts';

// ─── IMMUTABILITY ENFORCEMENT ────────────────────────────────

// Reject any attempt to modify an audit receipt
router.put('/receipt/:receiptId', (req, res) => {
  console.log(`[Audit] Rejected PUT attempt on receipt ${req.params.receiptId}`);
  res.status(403).json(IMMUTABILITY_ERROR);
});

router.patch('/receipt/:receiptId', (req, res) => {
  console.log(`[Audit] Rejected PATCH attempt on receipt ${req.params.receiptId}`);
  res.status(403).json(IMMUTABILITY_ERROR);
});

// Reject any attempt to delete an audit receipt
router.delete('/receipt/:receiptId', (req, res) => {
  console.log(`[Audit] Rejected DELETE attempt on receipt ${req.params.receiptId}`);
  res.status(403).json(IMMUTABILITY_ERROR);
});

// Reject bulk modification/deletion
router.delete('/:did/trail', (req, res) => {
  console.log(`[Audit] Rejected DELETE attempt on trail for ${req.params.did}`);
  res.status(403).json(IMMUTABILITY_ERROR);
});

router.put('/:did/trail', (req, res) => {
  console.log(`[Audit] Rejected PUT attempt on trail for ${req.params.did}`);
  res.status(403).json(IMMUTABILITY_ERROR);
});

router.patch('/:did/trail', (req, res) => {
  console.log(`[Audit] Rejected PATCH attempt on trail for ${req.params.did}`);
  res.status(403).json(IMMUTABILITY_ERROR);
});

// ─── READ ENDPOINTS ───────────────────────────────────────────

// GET /api/audit/me/trail - Get audit trail for authenticated subscriber
router.get('/me/trail', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;

    // Get subscriber DID
    const subResult = await db.query('SELECT did FROM subscribers WHERE id = $1', [req.user.id]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const did = subResult.rows[0].did;

    const result = await db.query(
      `${AUDIT_EVENT_SELECT} WHERE subject_did = $1 ORDER BY created_at DESC`,
      [did]
    );

    res.json(buildAuditTrailResponse(result.rows));
  } catch (err) {
    console.error('[Audit] Me trail error:', err.message);
    res.status(500).json({ error: 'Failed to get audit trail' });
  }
});

// Event type groups (mirrors frontend EVENT_TYPE_GROUPS)
const EVENT_TYPE_GROUPS = {
  scan: { label: 'Scan Events', types: ['scan_event', 'emergency_access'] },
  consent: { label: 'Consent Events', types: ['consent_granted', 'consent_revoked'] },
  access: { label: 'Provider Access', types: ['provider_data_access'] },
  record: { label: 'Record Events', types: ['record_upload', 'record_deleted', 'data_export'] },
  account: { label: 'Account Events', types: ['login', 'card_issued', 'claim_revoked'] },
};

// GET /api/audit/me/trail/export - Export audit trail as PDF for authenticated subscriber
// Supports ?eventType=<group> to export only filtered events
router.get('/me/trail/export', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;

    // Parse optional filter param
    const eventTypeParam = req.query.eventType || 'all';
    const filterGroup = EVENT_TYPE_GROUPS[eventTypeParam] || null;

    // Get subscriber details
    const subResult = await db.query(
      'SELECT id, did, first_name, last_name, email FROM subscribers WHERE id = $1',
      [req.user.id]
    );
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];
    const did = subscriber.did;

    // Get audit events ordered chronologically (oldest first for PDF)
    // Apply filter if requested
    let queryText;
    let queryParams;
    if (filterGroup) {
      queryText = `${AUDIT_EVENT_SELECT} WHERE subject_did = $1 AND event_type = ANY($2) ORDER BY created_at ASC`;
      queryParams = [did, filterGroup.types];
    } else {
      queryText = `${AUDIT_EVENT_SELECT} WHERE subject_did = $1 ORDER BY created_at ASC`;
      queryParams = [did];
    }
    const result = await db.query(queryText, queryParams);

    const events = buildAuditTrailResponse(result.rows);

    // Build content-disposition filename
    const filterSuffix = filterGroup ? `-${eventTypeParam}` : '';
    const dateStr = new Date().toISOString().split('T')[0];

    // Generate PDF
    const doc = new PDFDocument({ margin: 50, size: 'A4' });

    res.setHeader('Content-Type', 'application/pdf');
    res.setHeader(
      'Content-Disposition',
      `attachment; filename="audit-trail${filterSuffix}-${dateStr}.pdf"`
    );

    doc.pipe(res);

    // Header
    doc.fontSize(20).fillColor('#1e3a5f').text('LiveSafe Audit Trail', { align: 'center' });
    doc.moveDown(0.5);
    doc.fontSize(10).fillColor('#666').text(`Exported: ${new Date().toUTCString()}`, { align: 'center' });
    doc.moveDown(0.3);
    doc.fontSize(10).fillColor('#666').text(
      `Subscriber: ${subscriber.first_name || ''} ${subscriber.last_name || ''} <${subscriber.email}>`,
      { align: 'center' }
    );
    doc.moveDown(0.3);
    doc.fontSize(9).fillColor('#888').text(`DID: ${did}`, { align: 'center' });

    // Filter criteria note (if filtered export)
    if (filterGroup) {
      doc.moveDown(0.4);
      doc.fontSize(9).fillColor('#c05c00').text(
        `Filter: ${filterGroup.label} only (types: ${filterGroup.types.join(', ')})`,
        { align: 'center' }
      );
    }

    doc.moveDown(1);

    // Divider
    doc.moveTo(50, doc.y).lineTo(545, doc.y).stroke('#cccccc');
    doc.moveDown(0.5);

    // Summary
    const summaryLabel = filterGroup
      ? `Filtered Events (${filterGroup.label}): ${events.length}`
      : `Total Events: ${events.length}`;
    doc.fontSize(12).fillColor('#333').text(summaryLabel);
    doc.moveDown(1);

    if (events.length === 0) {
      doc.fontSize(11).fillColor('#666').text('No audit events found.', { align: 'center' });
    } else {
      // Events list
      events.forEach((event, idx) => {
        // Check if we need a new page
        if (doc.y > 700) {
          doc.addPage();
        }

        const eventDate = event.created_at
          ? new Date(event.created_at).toLocaleString('en-US', { timeZone: 'UTC' }) + ' UTC'
          : 'Unknown date';

        // Event header row
        doc.fontSize(10).fillColor('#1e3a5f')
          .text(`${idx + 1}. ${(event.event_type || 'unknown').replace(/_/g, ' ').toUpperCase()}`, {
            continued: true,
          })
          .fillColor('#888')
          .text(`  —  ${eventDate}`);

        // Actor / scope
        if (event.actor_did) {
          doc.fontSize(9).fillColor('#555').text(`  Actor: ${event.actor_did}`);
        }
        if (event.scope) {
          doc.fontSize(9).fillColor('#555').text(`  Scope: ${event.scope}`);
        }
        if (event.receipt_hash) {
          doc.fontSize(8).fillColor('#999').text(`  Receipt: ${event.receipt_hash}`);
        }
        if (event.details) {
          let detailsText = JSON.stringify(event.details);
          if (detailsText.length > 120) detailsText = detailsText.substring(0, 120) + '...';
          doc.fontSize(8).fillColor('#aaa').text(`  Details: ${detailsText}`);
        }

        doc.moveDown(0.5);

        // Light separator between events
        if (idx < events.length - 1) {
          doc.moveTo(60, doc.y).lineTo(535, doc.y).stroke('#eeeeee');
          doc.moveDown(0.3);
        }
      });
    }

    // Footer
    doc.moveDown(1);
    doc.moveTo(50, doc.y).lineTo(545, doc.y).stroke('#cccccc');
    doc.moveDown(0.5);
    doc.fontSize(8).fillColor('#999').text(
      LOCAL_AUDIT_IMMUTABILITY_NOTE,
      { align: 'center' }
    );

    doc.end();
    console.log(`[Audit] PDF export generated for subscriber ${req.user.id}, ${events.length} events`);
  } catch (err) {
    console.error('[Audit] PDF export error:', err.message);
    if (!res.headersSent) {
      res.status(500).json({ error: 'Failed to generate PDF export' });
    }
  }
});

// GET /api/audit/:did/trail - Get audit trail for a DID
// Requires authentication; subscribers can only access their own trail
router.get('/:did/trail', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { did } = req.params;

    // Feature #252: DID ownership check — subscribers can only view their own audit trail
    // Providers, admins, and trustees may access other subscribers' trails
    if (req.user.role === 'subscriber') {
      const tokenDid = req.user.did;
      if (!tokenDid || tokenDid !== did) {
        return res.status(403).json({
          error: 'Forbidden: subscribers can only access their own audit trail',
          code: 'CROSS_SUBSCRIBER_ACCESS_DENIED',
        });
      }
    }

    const result = await db.query(
      `${AUDIT_EVENT_SELECT} WHERE subject_did = $1 ORDER BY created_at DESC`,
      [did]
    );

    res.json(buildAuditTrailResponse(result.rows));
  } catch (err) {
    console.error('[Audit] Trail error:', err.message);
    res.status(500).json({ error: 'Failed to get audit trail' });
  }
});

// GET /api/audit/receipt/:receiptId - Get specific audit receipt
router.get('/receipt/:receiptId', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { receiptId } = req.params;

    const result = await db.query(
      `${AUDIT_EVENT_SELECT} WHERE id = $1`,
      [parseInt(receiptId)]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Audit receipt not found' });
    }

    res.json(buildAuditEventResponse(result.rows[0]));
  } catch (err) {
    console.error('[Audit] Receipt error:', err.message);
    res.status(500).json({ error: 'Failed to get audit receipt' });
  }
});

module.exports = router;
