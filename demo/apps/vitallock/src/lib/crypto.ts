// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

/**
 * VitalLock Crypto — Browser-side WASM wrapper for E2E encryption.
 *
 * All plaintext encryption/decryption happens in the browser via WASM.
 * The server never sees plaintext.
 */

import init, {
  wasm_caller_managed_x25519_public_from_secret,
  wasm_prepare_encrypted_message,
  wasm_attach_message_signature,
  wasm_decrypt_message,
  wasm_verify_message_signature,
  wasm_shamir_split,
  wasm_death_verification_initial_signing_payload,
  wasm_death_verification_new,
  wasm_death_verification_confirmation_signing_payload,
  wasm_death_verification_confirm,
} from '@/wasm/exochain_wasm';

let initialized = false;

/** Initialize the WASM module. Must be called before any crypto operation. */
export async function initCrypto(): Promise<void> {
  if (initialized) return;
  await init();
  initialized = true;
}

/** Check if WASM is initialized. */
export function isCryptoReady(): boolean {
  return initialized;
}

// ── Key Generation ──

export interface X25519KeyPair {
  public_key_hex: string;
  secret_key_hex: string;
}

function randomHex(byteLength: number): string {
  const bytes = new Uint8Array(byteLength);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function hexToBytes(hex: string): Uint8Array {
  if (hex.length % 2 !== 0 || !/^[0-9a-f]+$/iu.test(hex)) {
    throw new Error('invalid hex encoding');
  }
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

function arrayBufferFromBytes(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer as ArrayBuffer;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** Generate caller-managed X25519 material for encryption key exchange. */
export function generateX25519Keypair(): X25519KeyPair {
  const secretKeyHex = randomHex(32);
  const publicKey = wasm_caller_managed_x25519_public_from_secret(secretKeyHex);
  return {
    public_key_hex: publicKey.public_key_hex,
    secret_key_hex: secretKeyHex,
  };
}

// ── Message Encryption (E2E) ──

export interface EncryptedEnvelope {
  id: string;
  sender_did: string;
  recipient_did: string;
  ephemeral_public_key: number[];
  ciphertext: number[];
  content_type: string;
  signature: object;
  plaintext_hash: number[];
  release_on_death: boolean;
  release_delay_hours: number;
  created: { physical_ms: number; logical: number };
}

interface PreparedEnvelope {
  envelope: EncryptedEnvelope;
  signing_payload_hex: string;
}

async function signEnvelopePayload(
  senderEd25519PrivatePkcs8Hex: string,
  signingPayloadHex: string,
): Promise<string> {
  const privateKey = await crypto.subtle.importKey(
    'pkcs8',
    arrayBufferFromBytes(hexToBytes(senderEd25519PrivatePkcs8Hex)),
    { name: 'Ed25519' } as Algorithm,
    false,
    ['sign'],
  );
  const signature = await crypto.subtle.sign(
    { name: 'Ed25519' } as Algorithm,
    privateKey,
    arrayBufferFromBytes(hexToBytes(signingPayloadHex)),
  );
  return bytesToHex(new Uint8Array(signature));
}

/**
 * Lock & Send: encrypt a message client-side.
 * Plaintext never leaves the browser.
 */
export async function encryptMessage(
  plaintext: string,
  contentType: string,
  senderDid: string,
  recipientDid: string,
  senderEd25519PublicHex: string,
  senderEd25519PrivatePkcs8Hex: string,
  recipientX25519PublicHex: string,
  messageId: string,
  createdPhysicalMs: bigint,
  createdLogical: number,
  releaseOnDeath: boolean = false,
  releaseDelayHours: number = 0,
): Promise<EncryptedEnvelope> {
  const ephemeralKeypair = generateX25519Keypair();
  const prepared = wasm_prepare_encrypted_message(
    plaintext,
    JSON.stringify(contentType),
    senderDid,
    recipientDid,
    recipientX25519PublicHex,
    ephemeralKeypair.secret_key_hex,
    messageId,
    createdPhysicalMs,
    createdLogical,
    releaseOnDeath,
    releaseDelayHours,
  ) as PreparedEnvelope;
  const signatureHex = await signEnvelopePayload(
    senderEd25519PrivatePkcs8Hex,
    prepared.signing_payload_hex,
  );
  return wasm_attach_message_signature(
    JSON.stringify(prepared.envelope),
    senderEd25519PublicHex,
    signatureHex,
  ) as EncryptedEnvelope;
}

/**
 * Unlock: decrypt a message client-side.
 * Returns the plaintext string.
 */
export function decryptMessage(
  envelopeJson: string,
  recipientX25519SecretHex: string,
  senderEd25519PublicHex: string,
): { plaintext: string; content_type: string } {
  return wasm_decrypt_message(
    envelopeJson,
    recipientX25519SecretHex,
    senderEd25519PublicHex,
  );
}

/** Verify the sender's signature without decrypting. */
export function verifyMessageSignature(
  envelopeJson: string,
  senderEd25519PublicHex: string,
): boolean {
  return wasm_verify_message_signature(envelopeJson, senderEd25519PublicHex);
}

// ── Shamir Secret Sharing ──

export interface ShamirShare {
  index: number;
  data: number[];
  commitment: number[];
}

/**
 * Split a secret into Shamir shares.
 * Default: threshold=3, shares=4 (3-of-4 PACE).
 */
export function shamirSplit(
  secret: Uint8Array,
  threshold: number = 3,
  shares: number = 4,
): ShamirShare[] {
  return wasm_shamir_split(secret, threshold, shares);
}

// ── Death Verification ──

export interface DeathVerificationState {
  subject_did: string;
  initiated_by: string;
  required_confirmations: number;
  authorized_trustees: Record<string, number[]>;
  claim_nonce: number[];
  confirmations: Array<{
    trustee_did: string;
    public_key: number[];
    signature: { Ed25519: number[] } | { Hybrid: unknown } | { PostQuantum: number[] } | 'Empty';
    confirmed_at: object;
  }>;
  status: 'Pending' | 'Verified' | 'Rejected';
}

export interface AuthorizedDeathVerificationTrustee {
  did: string;
  public_key_hex: string;
}

/** Compute the bytes the initiating trustee must sign for a death claim. */
export function deathVerificationInitialSigningPayload(
  subjectDid: string,
  initiatedByDid: string,
  authorizedTrustees: AuthorizedDeathVerificationTrustee[],
  claimNonceHex: string,
  createdPhysicalMs: bigint,
  createdLogical: number,
  requiredConfirmations: number = 3,
): Uint8Array {
  return wasm_death_verification_initial_signing_payload(
    subjectDid,
    initiatedByDid,
    requiredConfirmations,
    JSON.stringify(authorizedTrustees),
    claimNonceHex,
    createdPhysicalMs,
    createdLogical,
  );
}

/** Create a new death verification request. */
export function createDeathVerification(
  subjectDid: string,
  initiatedByDid: string,
  authorizedTrustees: AuthorizedDeathVerificationTrustee[],
  claimNonceHex: string,
  initiatorSignatureHex: string,
  createdPhysicalMs: bigint,
  createdLogical: number,
  requiredConfirmations: number = 3,
): DeathVerificationState {
  return wasm_death_verification_new(
    subjectDid,
    initiatedByDid,
    requiredConfirmations,
    JSON.stringify(authorizedTrustees),
    claimNonceHex,
    initiatorSignatureHex,
    createdPhysicalMs,
    createdLogical,
  );
}

/** Compute the bytes a trustee must sign to confirm an existing death claim. */
export function deathVerificationConfirmationSigningPayload(
  stateJson: string,
  trusteeDid: string,
  confirmedPhysicalMs: bigint,
  confirmedLogical: number,
): Uint8Array {
  return wasm_death_verification_confirmation_signing_payload(
    stateJson,
    trusteeDid,
    confirmedPhysicalMs,
    confirmedLogical,
  );
}

/** Add a trustee confirmation to a death verification. */
export function confirmDeathVerification(
  stateJson: string,
  trusteeDid: string,
  trusteePublicKeyHex: string,
  signatureHex: string,
  confirmedPhysicalMs: bigint,
  confirmedLogical: number,
): { verified: boolean; confirmations_remaining: number; state: DeathVerificationState } {
  return wasm_death_verification_confirm(
    stateJson,
    trusteeDid,
    trusteePublicKeyHex,
    signatureHex,
    confirmedPhysicalMs,
    confirmedLogical,
  );
}
