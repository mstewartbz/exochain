import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');

function readSource(path) {
  return readFileSync(resolve(root, path), 'utf8');
}

const login = readSource('src/pages/Login.tsx');
const settings = readSource('src/pages/Settings.tsx');
const compose = readSource('src/pages/Compose.tsx');
const auth = readSource('src/hooks/useAuth.ts');
const crypto = readSource('src/lib/crypto.ts');
const vault = readSource('src/lib/localVault.ts');
const api = readSource('src/lib/api.ts');
const service = readSource('../../services/vitallock-api/src/index.js');

assert(
  !login.includes('ed25519PublicHex = ed25519SecretHex'),
  'VitalLock login must not publish the passphrase-derived signing seed as an Ed25519 public key',
);

assert(
  !settings.includes('auth?.ed25519SecretHex') && !settings.includes('auth?.ed25519PublicHex'),
  'VitalLock settings must not display or copy passphrase-derived Ed25519 key material',
);

assert(
  !compose.includes('auth!.ed25519SecretHex') && !compose.includes('auth.ed25519SecretHex'),
  'VitalLock compose must not pass a session-stored passphrase hash as a signing secret',
);

assert(
  !auth.includes('ed25519SecretHex: string;') && !login.includes('ed25519PublicHex = ed25519SecretHex'),
  'VitalLock auth state must not persist passphrase-derived Ed25519 key material',
);

assert(
  !crypto.includes('wasm_ed25519_public_from_secret') && !crypto.includes('senderSigningKeyHex'),
  'VitalLock crypto wrapper must not expose raw-secret public derivation or raw signing-key parameters',
);

assert(
  !login.includes("crypto.subtle.digest('SHA-256', encoder.encode(passphrase))")
    && !login.includes('identityDigestHex')
    && !login.includes('Derive DID from passphrase hash'),
  'VitalLock login must not derive the DID or display identity from an unsalted passphrase hash',
);

assert(
  vault.includes('PBKDF2')
    && vault.includes('AES-GCM')
    && vault.includes('localStorage.setItem')
    && vault.includes('crypto.getRandomValues')
    && vault.includes("crypto.subtle.generateKey")
    && vault.includes('did: `did:exo:${didRandomHex}`'),
  'VitalLock local identity must be random and persisted only through an encrypted browser-local vault',
);

assert(
  crypto.includes('wasm_prepare_encrypted_message')
    && crypto.includes('wasm_attach_message_signature')
    && !crypto.includes('wasm_encrypt_message'),
  'VitalLock client encryption must use prepare/sign/attach instead of the raw-signing WASM entrypoint',
);

assert(
  !api.includes('sender_signing_key_hex: string')
    && !service.includes('wasm.wasm_encrypt_message(')
    && !service.includes('wasm.wasm_generate_x25519_keypair()'),
  'VitalLock API/service contracts must not accept raw signing keys or generate private messaging keys server-side',
);
