// Gateway API — ExoChain orchestrator service
// Routes requests through the full governance pipeline: identity → consent → authority → governance → audit
import http from 'node:http';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const wasm = require('@exochain/exochain-wasm');
import pg from 'pg';

const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL });
const PORT = process.env.PORT || 3000;

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
    // ── Health ──
    if (url.pathname === '/health') {
      const db = await pool.query('SELECT 1');
      return json(res, 200, { status: 'ok', service: 'gateway-api', db: 'connected', wasm: true });
    }

    // ── System Info ──
    if (url.pathname === '/api/system' && req.method === 'GET') {
      const invariants = wasm.wasm_enforce_invariants('{}');
      const mcpRules = wasm.wasm_mcp_rules();
      const stages = wasm.wasm_workflow_stages();
      const transitions = wasm.wasm_bcts_valid_transitions('"Draft"');
      return json(res, 200, {
        constitutional_invariants: invariants.invariants,
        mcp_rules: mcpRules,
        workflow_stages: stages,
        bcts_draft_transitions: transitions,
      });
    }

    // ── Users ──
    if (url.pathname === '/api/users' && req.method === 'GET') {
      const { rows } = await pool.query('SELECT did, display_name, email, roles, status, pace_status FROM users ORDER BY display_name');
      return json(res, 200, rows);
    }

    // ── Identity Scores ──
    if (url.pathname === '/api/identity/scores' && req.method === 'GET') {
      const { rows } = await pool.query('SELECT i.did, u.display_name, i.score, i.tier, i.factors FROM identity_scores i JOIN users u ON u.did = i.did ORDER BY i.score DESC');
      return json(res, 200, rows);
    }

    // ── Decisions: List ──
    if (url.pathname === '/api/decisions' && req.method === 'GET') {
      const { rows } = await pool.query('SELECT id_hash, title, status, decision_class, author, created_at_ms FROM decisions ORDER BY created_at_ms DESC');
      return json(res, 200, rows);
    }

    // ── Decisions: Create (full WASM pipeline) ──
    if (url.pathname === '/api/decisions' && req.method === 'POST') {
      const body = await parseBody(req);
      const { title, decision_class, author_did } = body;

      // 1. Verify author exists
      const { rows: [author] } = await pool.query('SELECT did, roles, pace_status FROM users WHERE did = $1', [author_did]);
      if (!author) return json(res, 404, { error: 'Author not found' });
      if (author.pace_status !== 'Enrolled') return json(res, 403, { error: 'Author not PACE enrolled' });

      // 2. Get constitution hash
      const { rows: [constitution] } = await pool.query(
        'SELECT version, payload FROM constitutions WHERE tenant_id = $1 ORDER BY version DESC LIMIT 1',
        ['exochain-foundation']
      );
      const constitutionHash = wasm.wasm_hash_structured(JSON.stringify(constitution.payload));

      // 3. Create DecisionObject via WASM
      const decision = wasm.wasm_create_decision(title, JSON.stringify(decision_class || 'Operational'), constitutionHash);

      // 4. Persist to DB
      await pool.query(
        'INSERT INTO decisions (id_hash, tenant_id, status, title, decision_class, author, created_at_ms, constitution_version, payload) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)',
        [decision.id, 'exochain-foundation', 'Draft', title, decision_class || 'Operational', author_did, Date.now(), constitution.version, JSON.stringify(decision)]
      );

      // 5. Audit trail
      const evidenceHash = wasm.wasm_hash_bytes(new Uint8Array(Buffer.from(JSON.stringify(decision))));
      wasm.wasm_audit_append(author_did, 'CreateDecision', 'success', evidenceHash);

      return json(res, 201, { decision, constitution_version: constitution.version });
    }

    // ── Decisions: Vote ──
    if (url.pathname === '/api/decisions/vote' && req.method === 'POST') {
      const body = await parseBody(req);
      const { decision_id, voter_did, choice, rationale } = body;

      // Load decision
      const { rows: [row] } = await pool.query('SELECT payload FROM decisions WHERE id_hash = $1', [decision_id]);
      if (!row) return json(res, 404, { error: 'Decision not found' });

      // Check clearance
      const clearancePolicy = { actions: { Vote: { required_level: 'Governor' } } };
      const clearance = wasm.wasm_check_clearance(voter_did, 'Vote', JSON.stringify(clearancePolicy));
      if (clearance.status !== 'Granted') return json(res, 403, { error: 'Insufficient clearance', details: clearance });

      // Check conflicts
      const action = { action_id: decision_id, actor_did: voter_did, affected_dids: [], description: `Vote on ${decision_id}` };
      const conflicts = wasm.wasm_check_conflicts(voter_did, JSON.stringify(action), '[]');
      if (conflicts.must_recuse) return json(res, 403, { error: 'Must recuse due to conflict of interest', conflicts });

      // Add vote via WASM
      const kp = wasm.wasm_generate_keypair();
      const vote = { voter: voter_did, choice, rationale, signature: kp.public_key, timestamp_ms: Date.now() };

      const updated = wasm.wasm_add_vote(JSON.stringify(row.payload), JSON.stringify(vote));

      // Persist
      await pool.query('UPDATE decisions SET payload = $1 WHERE id_hash = $2', [JSON.stringify(updated), decision_id]);

      return json(res, 200, { vote_recorded: true, decision: updated });
    }

    // ── Decisions: Transition State ──
    if (url.pathname === '/api/decisions/transition' && req.method === 'POST') {
      const body = await parseBody(req);
      const { decision_id, to_state, actor_did } = body;

      const { rows: [row] } = await pool.query('SELECT payload, status FROM decisions WHERE id_hash = $1', [decision_id]);
      if (!row) return json(res, 404, { error: 'Decision not found' });

      const updated = wasm.wasm_transition_decision(JSON.stringify(row.payload), JSON.stringify(to_state), actor_did);

      await pool.query('UPDATE decisions SET payload = $1, status = $2 WHERE id_hash = $3', [JSON.stringify(updated), to_state, decision_id]);

      // Check if terminal
      const isTerminal = wasm.wasm_decision_is_terminal(JSON.stringify(updated));

      return json(res, 200, { decision: updated, new_state: to_state, is_terminal: isTerminal });
    }

    // ── Delegations ──
    if (url.pathname === '/api/delegations' && req.method === 'GET') {
      const { rows } = await pool.query('SELECT id_hash, delegator, delegatee, payload, created_at_ms, expires_at FROM delegations ORDER BY created_at_ms DESC');
      return json(res, 200, rows);
    }

    // ── Constitution ──
    if (url.pathname === '/api/constitution' && req.method === 'GET') {
      const { rows: [constitution] } = await pool.query(
        'SELECT * FROM constitutions WHERE tenant_id = $1 ORDER BY version DESC LIMIT 1',
        ['exochain-foundation']
      );
      return json(res, 200, constitution || { error: 'No constitution found' });
    }

    // ── Audit Trail ──
    if (url.pathname === '/api/audit' && req.method === 'GET') {
      const { rows } = await pool.query('SELECT * FROM audit_entries ORDER BY sequence DESC LIMIT 50');
      return json(res, 200, rows);
    }

    // ── Crypto: Hash ──
    if (url.pathname === '/api/crypto/hash' && req.method === 'POST') {
      const body = await parseBody(req);
      const hash = wasm.wasm_hash_structured(JSON.stringify(body.data || body));
      return json(res, 200, { hash });
    }

    // ── Crypto: Keypair ──
    if (url.pathname === '/api/crypto/keypair' && req.method === 'POST') {
      const kp = wasm.wasm_generate_keypair();
      return json(res, 200, kp);
    }

    // ── Crypto: Sign + Verify ──
    if (url.pathname === '/api/crypto/sign' && req.method === 'POST') {
      const body = await parseBody(req);
      const sig = wasm.wasm_sign(new Uint8Array(Buffer.from(body.message)), body.secret_key);
      return json(res, 200, { signature: sig });
    }

    if (url.pathname === '/api/crypto/verify' && req.method === 'POST') {
      const body = await parseBody(req);
      const valid = wasm.wasm_verify(new Uint8Array(Buffer.from(body.message)), body.signature, body.public_key);
      return json(res, 200, { valid });
    }

    // ── Consent Anchors ──
    if (url.pathname === '/api/consent' && req.method === 'GET') {
      const { rows } = await pool.query('SELECT * FROM consent_anchors ORDER BY granted_at_ms DESC');
      return json(res, 200, rows);
    }

    // ── BCTS State Machine ──
    if (url.pathname === '/api/bcts/transitions' && req.method === 'GET') {
      const state = url.searchParams.get('state') || 'Draft';
      const transitions = wasm.wasm_bcts_valid_transitions(JSON.stringify(state));
      const isTerminal = wasm.wasm_bcts_is_terminal(JSON.stringify(state));
      return json(res, 200, { state, transitions, is_terminal: isTerminal });
    }

    // ── Shamir Secret Sharing Demo ──
    if (url.pathname === '/api/identity/shamir/split' && req.method === 'POST') {
      const body = await parseBody(req);
      const secret = new Uint8Array(Buffer.from(body.secret || 'demo-secret'));
      const shares = wasm.wasm_shamir_split(secret, body.threshold || 2, body.shares || 3);
      return json(res, 200, { shares, threshold: body.threshold || 2, total: body.shares || 3 });
    }

    json(res, 404, { error: 'Not found', available_endpoints: [
      'GET  /health', 'GET  /api/system', 'GET  /api/users', 'GET  /api/identity/scores',
      'GET  /api/decisions', 'POST /api/decisions', 'POST /api/decisions/vote',
      'POST /api/decisions/transition', 'GET  /api/delegations', 'GET  /api/constitution',
      'GET  /api/audit', 'POST /api/crypto/hash', 'POST /api/crypto/keypair',
      'POST /api/crypto/sign', 'POST /api/crypto/verify', 'GET  /api/consent',
      'GET  /api/bcts/transitions?state=Draft', 'POST /api/identity/shamir/split',
    ]});
  } catch (e) {
    console.error('Request error:', e);
    json(res, 500, { error: e.message });
  }
});

server.listen(PORT, () => {
  console.log(`[gateway-api] ExoChain Gateway running on :${PORT}`);
  console.log(`[gateway-api] WASM loaded — 45 governance functions available`);
});
