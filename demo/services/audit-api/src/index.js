// Audit API — hash-chained audit log, verification, provenance
import http from 'node:http';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const wasm = require('@exochain/exochain-wasm');
import pg from 'pg';

const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL });
const PORT = process.env.PORT || 3007;

function json(res, status, data) {
  res.writeHead(status, { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': 'Content-Type' });
  res.end(JSON.stringify(data, null, 2));
}

function parseBody(req) {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', c => body += c);
    req.on('end', () => { try { resolve(body ? JSON.parse(body) : {}); } catch(e) { reject(e); } });
  });
}

const server = http.createServer(async (req, res) => {
  if (req.method === 'OPTIONS') {
    res.writeHead(204, { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Methods': '*', 'Access-Control-Allow-Headers': 'Content-Type' });
    return res.end();
  }
  const url = new URL(req.url, `http://${req.headers.host}`);

  try {
    if (url.pathname === '/health') return json(res, 200, { status: 'ok', service: 'audit-api' });

    // ── List Audit Entries ──
    if (url.pathname === '/api/entries' && req.method === 'GET') {
      const limit = url.searchParams.get('limit') || 50;
      const { rows } = await pool.query('SELECT * FROM audit_entries ORDER BY sequence DESC LIMIT $1', [limit]);
      return json(res, 200, rows);
    }

    // ── Append Audit Entry ──
    if (url.pathname === '/api/entries' && req.method === 'POST') {
      const { actor_did, action, result, evidence_hash } = await parseBody(req);
      const auditResult = wasm.wasm_audit_append(actor_did, action, result, evidence_hash || '0'.repeat(64));

      // Also persist to DB
      const { rows: [last] } = await pool.query('SELECT COALESCE(MAX(sequence), -1) as seq, COALESCE(MAX(entry_hash), $1) as prev FROM audit_entries', ['0'.repeat(64)]);
      const eventHash = wasm.wasm_hash_bytes(new Uint8Array(Buffer.from(`${action}:${result}`)));

      await pool.query(
        'INSERT INTO audit_entries (sequence, prev_hash, event_hash, event_type, actor, tenant_id, timestamp_physical_ms, entry_hash) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)',
        [last.seq + 1, last.prev, eventHash, action, actor_did, 'exochain-foundation', Date.now(), auditResult.head_hash]
      );

      return json(res, 201, { entry_count: auditResult.entries, head_hash: auditResult.head_hash });
    }

    // ── Verify Chain Integrity ──
    if (url.pathname === '/api/verify' && req.method === 'GET') {
      const { rows } = await pool.query('SELECT * FROM audit_entries ORDER BY sequence ASC');

      // Verify hash chain manually
      let valid = true;
      let error = null;
      for (let i = 1; i < rows.length; i++) {
        if (rows[i].prev_hash !== rows[i - 1].entry_hash) {
          valid = false;
          error = `Chain break at sequence ${rows[i].sequence}: prev_hash doesn't match previous entry_hash`;
          break;
        }
      }

      return json(res, 200, {
        intact: valid,
        entries_checked: rows.length,
        head_hash: rows.length > 0 ? rows[rows.length - 1].entry_hash : null,
        error,
      });
    }

    json(res, 404, { error: 'Not found' });
  } catch (e) {
    console.error('Error:', e);
    json(res, 500, { error: e.message });
  }
});

server.listen(PORT, () => console.log(`[audit-api] Running on :${PORT}`));
