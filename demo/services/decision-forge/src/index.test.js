import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';
import supertest from 'supertest';

const mockWasm = vi.hoisted(() => ({
  wasm_create_decision: vi.fn((title, decClass, hash) => ({
    id: 'test-dec-001', title, decision_class: JSON.parse(decClass), constitution_hash: hash, status: 'Draft', votes: [], evidence: [],
  })),
  wasm_add_vote: vi.fn((decJson, voteJson) => ({ ...JSON.parse(decJson), votes: [...(JSON.parse(decJson).votes || []), JSON.parse(voteJson)] })),
  wasm_add_evidence: vi.fn((decJson, evJson) => ({ ...JSON.parse(decJson), evidence: [...(JSON.parse(decJson).evidence || []), JSON.parse(evJson)] })),
  wasm_transition_decision: vi.fn((decJson, stateJson) => ({ ...JSON.parse(decJson), status: JSON.parse(stateJson) })),
  wasm_decision_content_hash: vi.fn(() => 'c'.repeat(64)),
  wasm_decision_is_terminal: vi.fn(() => false),
  wasm_file_challenge: vi.fn((challenger, id, ground) => ({ challenger_did: challenger, target_id: id, ground: JSON.parse(ground), challenge_hash: 'd'.repeat(64) })),
  wasm_propose_accountability: vi.fn((target, proposer, actionType, reason, evidence) => ({ target_did: target, proposer_did: proposer, action_type: JSON.parse(actionType), reason, evidence_hash: evidence })),
  wasm_workflow_stages: vi.fn(() => ['Draft', 'Review', 'Voting', 'Enacted', 'Withdrawn']),
}));

vi.mock('module', async (importOriginal) => {
  const orig = await importOriginal();
  return { ...orig, createRequire: () => (id) => { if (id === '@exochain/exochain-wasm') return mockWasm; throw new Error(`Unexpected require('${id}') in test`); } };
});

vi.mock('pg', () => { const q = vi.fn(); const P = vi.fn(() => ({ query: q })); return { default: { Pool: P } }; });

import { server } from './index.js';

let request;
beforeAll(async () => { await new Promise((r) => server.listen(0, r)); request = supertest(server); });
afterAll(async () => { await new Promise((r) => server.close(r)); });

describe('GET /health', () => {
  it('returns 200', async () => {
    const res = await request.get('/health');
    expect(res.status).toBe(200);
    expect(res.body.service).toBe('decision-forge');
  });
});

describe('POST /api/decision/create', () => {
  it('creates a DecisionObject via WASM', async () => {
    const res = await request.post('/api/decision/create').send({ title: 'Adopt RFC-42', decision_class: 'Strategic', constitution_hash: 'a'.repeat(64) });
    expect(res.status).toBe(201);
    expect(res.body).toHaveProperty('id', 'test-dec-001');
    expect(res.body.title).toBe('Adopt RFC-42');
    expect(res.body.status).toBe('Draft');
  });

  it('defaults decision_class to Operational', async () => {
    const res = await request.post('/api/decision/create').send({ title: 'Simple Decision' });
    expect(res.status).toBe(201);
    expect(res.body.decision_class).toBe('Operational');
  });
});

describe('POST /api/decision/vote', () => {
  it('adds a vote to the decision', async () => {
    const res = await request.post('/api/decision/vote').send({ decision_json: { id: 'test-dec-001', title: 'Test', status: 'Draft', votes: [] }, vote: { voter: 'did:exo:alice', choice: 'Approve' } });
    expect(res.status).toBe(200);
    expect(res.body.votes).toHaveLength(1);
  });
});

describe('POST /api/decision/evidence', () => {
  it('attaches evidence to the decision', async () => {
    const res = await request.post('/api/decision/evidence').send({ decision_json: { id: 'test-dec-001', evidence: [] }, evidence: { hash: 'e'.repeat(64), description: 'doc' } });
    expect(res.status).toBe(200);
    expect(res.body.evidence).toHaveLength(1);
  });
});

describe('POST /api/decision/transition', () => {
  it('transitions decision to new state', async () => {
    const res = await request.post('/api/decision/transition').send({ decision_json: { id: 'test-dec-001', status: 'Draft' }, to_state: 'Review', actor_did: 'did:exo:alice' });
    expect(res.status).toBe(200);
    expect(res.body.status).toBe('Review');
  });
});

describe('POST /api/decision/hash', () => {
  it('returns content hash', async () => {
    const res = await request.post('/api/decision/hash').send({ decision_json: { id: 'test-dec-001', status: 'Draft' } });
    expect(res.status).toBe(200);
    expect(res.body.hash).toBe('c'.repeat(64));
  });
});

describe('POST /api/decision/terminal', () => {
  it('returns is_terminal flag', async () => {
    const res = await request.post('/api/decision/terminal').send({ decision_json: { id: 'test-dec-001', status: 'Draft' } });
    expect(res.status).toBe(200);
    expect(res.body).toHaveProperty('is_terminal', false);
  });
});

describe('POST /api/decision/challenge', () => {
  it('files a governance challenge', async () => {
    const res = await request.post('/api/decision/challenge').send({ challenger_did: 'did:exo:carol', decision_id: 'test-dec-001', ground: 'ConstitutionalViolation', evidence_hash: 'e'.repeat(64) });
    expect(res.status).toBe(200);
    expect(res.body).toHaveProperty('challenger_did', 'did:exo:carol');
  });
});

describe('POST /api/decision/accountability', () => {
  it('proposes accountability action', async () => {
    const res = await request.post('/api/decision/accountability').send({ target_did: 'did:exo:dave', proposer_did: 'did:exo:alice', action_type: 'Censure', reason: 'Violation', evidence_hash: 'f'.repeat(64) });
    expect(res.status).toBe(200);
    expect(res.body).toHaveProperty('target_did', 'did:exo:dave');
  });
});

describe('GET /api/workflow/stages', () => {
  it('returns workflow stage list', async () => {
    const res = await request.get('/api/workflow/stages');
    expect(res.status).toBe(200);
    expect(Array.isArray(res.body.stages)).toBe(true);
    expect(res.body.stages).toContain('Draft');
  });
});

describe('OPTIONS preflight', () => {
  it('returns 204', async () => { expect((await request.options('/api/decision/create')).status).toBe(204); });
});

describe('Unknown route', () => {
  it('returns 404', async () => { expect((await request.get('/api/nonexistent')).status).toBe(404); });
});
