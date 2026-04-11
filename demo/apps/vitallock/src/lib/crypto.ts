/**
 * VitalLock Crypto — Browser-side WASM wrapper for E2E encryption.
 *
 * All plaintext encryption/decryption happens in the browser via WASM.
 * The server never sees plaintext.
 */

import init, {
  wasm_generate_keypair,
  wasm_generate_x25519_keypair,
  wasm_encrypt_message,
  wasm_decrypt_message,
  wasm_verify_message_signature,
  wasm_shamir_split,
  wasm_death_verification_new,
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

export interface Ed25519KeyPair {
  public_key_hex: string;
  secret_key_hex: string;
}

export interface X25519KeyPair {
  public_key_hex: string;
  secret_key_hex: string;
}

/** Generate an Ed25519 keypair (for signing). */
export function generateEd25519Keypair(): Ed25519KeyPair {
  return wasm_generate_keypair();
}

/** Generate an X25519 keypair (for encryption key exchange). */
export function generateX25519Keypair(): X25519KeyPair {
  return wasm_generate_x25519_keypair();
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

/**
 * Lock & Send: encrypt a message client-side.
 * Plaintext never leaves the browser.
 */
export function encryptMessage(
  plaintext: string,
  contentType: string,
  senderDid: string,
  recipientDid: string,
  senderSigningKeyHex: string,
  recipientX25519PublicHex: string,
  releaseOnDeath: boolean = false,
  releaseDelayHours: number = 0,
): EncryptedEnvelope {
  return wasm_encrypt_message(
    plaintext,
    JSON.stringify(contentType),
    senderDid,
    recipientDid,
    senderSigningKeyHex,
    recipientX25519PublicHex,
    releaseOnDeath,
    releaseDelayHours,
  );
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
  confirmations: Array<{ trustee_did: string; confirmed_at: object }>;
  status: 'Pending' | 'Verified' | 'Rejected';
}

/** Create a new death verification request. */
export function createDeathVerification(
  subjectDid: string,
  initiatedByDid: string,
  requiredConfirmations: number = 3,
): DeathVerificationState {
  return wasm_death_verification_new(subjectDid, initiatedByDid, requiredConfirmations);
}

/** Add a trustee confirmation to a death verification. */
export function confirmDeathVerification(
  stateJson: string,
  trusteeDid: string,
): { verified: boolean; confirmations_remaining: number; state: DeathVerificationState } {
  return wasm_death_verification_confirm(stateJson, trusteeDid);
}
