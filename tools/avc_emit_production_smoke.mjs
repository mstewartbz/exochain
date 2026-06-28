#!/usr/bin/env node
import { readFile } from 'node:fs/promises';

const DEFAULT_BASE_URL = 'https://exochain.io';
const DEFAULT_EXPECTED_CASES = 18;
const DEFAULT_ROUNDS = 2;

function fail(message) {
  console.error(`AVC production smoke failed: ${message}`);
  process.exit(1);
}

function requiredEnv(name) {
  const value = process.env[name];
  if (typeof value !== 'string' || value.trim().length === 0) {
    fail(`${name} must be set`);
  }
  return value;
}

function optionalPositiveInteger(name, fallback) {
  const raw = process.env[name];
  if (raw === undefined || raw.trim().length === 0) {
    return fallback;
  }
  const parsed = Number(raw);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    fail(`${name} must be a positive safe integer`);
  }
  return parsed;
}

async function readJson(path, envName) {
  let parsed;
  try {
    parsed = JSON.parse(await readFile(path, 'utf8'));
  } catch (error) {
    fail(`${envName} could not be read as JSON: ${error.message}`);
  }
  return parsed;
}

function requestArray(parsed, envName) {
  const requests = Array.isArray(parsed) ? parsed : parsed?.requests;
  if (!Array.isArray(requests)) {
    fail(`${envName} must contain a JSON array or an object with a requests array`);
  }
  if (requests.length === 0) {
    fail(`${envName} must contain at least one request`);
  }
  for (let index = 0; index < requests.length; index += 1) {
    const item = requests[index];
    if (typeof item !== 'object' || item === null || Array.isArray(item)) {
      fail(`${envName}[${index}] must be a JSON object`);
    }
  }
  return requests;
}

async function requestJson(baseUrl, path, token, method, body) {
  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers: {
      accept: 'application/json',
      authorization: `Bearer ${token}`,
      'content-type': 'application/json',
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let payload = undefined;
  if (text.length > 0) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = text;
    }
  }
  if (!response.ok) {
    const detail = typeof payload === 'string' ? payload : JSON.stringify(payload);
    throw new Error(`HTTP ${response.status} for ${method} ${path}: ${detail}`);
  }
  return payload;
}

function requireString(value, name) {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`${name} must be a non-empty string`);
  }
  return value;
}

function requirePresent(value, name) {
  if (value === undefined || value === null) {
    throw new Error(`${name} must be present`);
  }
  return value;
}

function validateReceiptProof(receipt, context, allowEmptyPreviousReceipt) {
  requirePresent(receipt, `${context}.receipt`);
  requirePresent(receipt.external_timestamp_proof, `${context}.external_timestamp_proof`);
  requirePresent(receipt.action_descriptor, `${context}.action_descriptor`);
  requireString(receipt.action_descriptor_hash, `${context}.action_descriptor_hash`);
  if (!allowEmptyPreviousReceipt) {
    requireString(receipt.previous_receipt_hash, `${context}.previous_receipt_hash`);
  }
  if (receipt.timestamp_provenance !== 'ExternalTimestampAuthority') {
    throw new Error(`${context}.timestamp_provenance must be ExternalTimestampAuthority`);
  }
}

function validateEmitResponse(payload, context, allowEmptyPreviousReceipt) {
  const receiptHash = requireString(payload?.receipt_hash, `${context}.receipt_hash`);
  requireString(payload.exochain_finality_hash, `${context}.exochain_finality_hash`);
  requirePresent(payload.exochain_finality_height, `${context}.exochain_finality_height`);
  requireString(
    payload.exochain_finality_receipt_hash,
    `${context}.exochain_finality_receipt_hash`,
  );
  validateReceiptProof(payload.receipt, `${context}.receipt`, allowEmptyPreviousReceipt);
  return receiptHash;
}

async function seedCredentials(baseUrl, token, issueRequests) {
  for (let index = 0; index < issueRequests.length; index += 1) {
    const payload = await requestJson(baseUrl, '/api/v1/avc/issue', token, 'POST', issueRequests[index]);
    if (payload?.status !== 'registered') {
      throw new Error(`seed credential ${index + 1} did not return status=registered`);
    }
    console.log(`seed ${index + 1}/${issueRequests.length}: ${payload.credential_id}`);
  }
}

async function run() {
  const baseUrl = (process.env.EXO_AVC_SMOKE_BASE_URL ?? DEFAULT_BASE_URL).replace(/\/+$/, '');
  const token = requiredEnv('EXO_AVC_SMOKE_BEARER_TOKEN');
  const emitRequestsPath = requiredEnv('EXO_AVC_SMOKE_EMIT_REQUESTS_FILE');
  const expectedCases = optionalPositiveInteger('EXO_AVC_SMOKE_EXPECTED_CASES', DEFAULT_EXPECTED_CASES);
  const rounds = optionalPositiveInteger('EXO_AVC_SMOKE_ROUNDS', DEFAULT_ROUNDS);
  const allowEmptyPreviousReceipt = process.env.EXO_AVC_SMOKE_ALLOW_EMPTY_PREVIOUS_RECEIPT === '1';

  const emitRequests = requestArray(
    await readJson(emitRequestsPath, 'EXO_AVC_SMOKE_EMIT_REQUESTS_FILE'),
    'EXO_AVC_SMOKE_EMIT_REQUESTS_FILE',
  );
  if (emitRequests.length !== expectedCases) {
    fail(`expected ${expectedCases} emit requests, got ${emitRequests.length}`);
  }

  const issueRequestsPath = process.env.EXO_AVC_SMOKE_ISSUE_REQUESTS_FILE;
  if (issueRequestsPath !== undefined && issueRequestsPath.trim().length > 0) {
    const issueRequests = requestArray(
      await readJson(issueRequestsPath, 'EXO_AVC_SMOKE_ISSUE_REQUESTS_FILE'),
      'EXO_AVC_SMOKE_ISSUE_REQUESTS_FILE',
    );
    await seedCredentials(baseUrl, token, issueRequests);
  }

  let successCount = 0;
  for (let round = 1; round <= rounds; round += 1) {
    for (let index = 0; index < emitRequests.length; index += 1) {
      const context = `round ${round} case ${index + 1}`;
      const emitResponse = await requestJson(
        baseUrl,
        '/api/v1/avc/receipts/emit',
        token,
        'POST',
        emitRequests[index],
      );
      const receiptHash = validateEmitResponse(emitResponse, context, allowEmptyPreviousReceipt);
      const fetched = await requestJson(
        baseUrl,
        `/api/v1/avc/receipts/${encodeURIComponent(receiptHash)}`,
        token,
        'GET',
      );
      validateReceiptProof(fetched, `${context} fetched receipt`, allowEmptyPreviousReceipt);
      successCount += 1;
      console.log(`${context}: OK receipt_hash=${receiptHash}`);
    }
  }

  console.log(`AVC production smoke passed: ${successCount}/${rounds * emitRequests.length} emits`);
}

run().catch((error) => {
  fail(error instanceof Error ? error.message : String(error));
});
