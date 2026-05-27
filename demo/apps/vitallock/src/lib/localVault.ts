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

import { generateX25519Keypair } from '@/lib/crypto';

const VAULT_STORAGE_KEY = 'vitallock_local_vault_v1';
const VAULT_VERSION = 1;
const VAULT_KDF = 'PBKDF2-SHA256-AES-GCM';
const VAULT_ITERATIONS = 210_000;
const VAULT_SALT_BYTES = 16;
const VAULT_IV_BYTES = 12;
const DID_RANDOM_BYTES = 16;

interface SealedVault {
  version: typeof VAULT_VERSION;
  kdf: typeof VAULT_KDF;
  iterations: typeof VAULT_ITERATIONS;
  saltHex: string;
  ivHex: string;
  ciphertextHex: string;
}

export interface LocalVaultIdentity {
  did: string;
  displayName: string;
  ed25519PublicHex: string;
  ed25519PrivatePkcs8Hex: string;
  x25519PublicHex: string;
  x25519SecretHex: string;
}

function randomBytes(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function arrayBufferFromBytes(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer as ArrayBuffer;
}

async function generateEd25519SigningMaterial(): Promise<{
  publicKeyHex: string;
  privatePkcs8Hex: string;
}> {
  const keypair = await crypto.subtle.generateKey(
    { name: 'Ed25519' } as Algorithm,
    true,
    ['sign', 'verify'],
  ) as CryptoKeyPair;
  const publicKey = await crypto.subtle.exportKey('raw', keypair.publicKey);
  const privateKey = await crypto.subtle.exportKey('pkcs8', keypair.privateKey);
  return {
    publicKeyHex: bytesToHex(new Uint8Array(publicKey)),
    privatePkcs8Hex: bytesToHex(new Uint8Array(privateKey)),
  };
}

function hexToBytes(hex: string): Uint8Array {
  if (hex.length % 2 !== 0 || !/^[0-9a-f]+$/iu.test(hex)) {
    throw new Error('invalid hex encoding in local vault');
  }
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

async function deriveVaultKey(passphrase: string, salt: Uint8Array): Promise<CryptoKey> {
  const encodedPassphrase = new TextEncoder().encode(passphrase);
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encodedPassphrase,
    'PBKDF2',
    false,
    ['deriveKey'],
  );
  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: arrayBufferFromBytes(salt),
      iterations: VAULT_ITERATIONS,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  );
}

function sealToStorage(sealed: SealedVault): void {
  localStorage.setItem(VAULT_STORAGE_KEY, JSON.stringify(sealed));
}

function loadSealedVault(): SealedVault {
  const raw = localStorage.getItem(VAULT_STORAGE_KEY);
  if (!raw) {
    throw new Error('No local VitalLock vault found. Create an account on this browser first.');
  }
  const parsed = JSON.parse(raw) as SealedVault;
  if (
    parsed.version !== VAULT_VERSION
    || parsed.kdf !== VAULT_KDF
    || parsed.iterations !== VAULT_ITERATIONS
    || typeof parsed.saltHex !== 'string'
    || typeof parsed.ivHex !== 'string'
    || typeof parsed.ciphertextHex !== 'string'
  ) {
    throw new Error('Local VitalLock vault format is unsupported.');
  }
  return parsed;
}

function defaultDisplayName(randomDidHex: string, displayName: string): string {
  const trimmed = displayName.trim();
  return trimmed || `User-${randomDidHex.slice(0, 8)}`;
}

async function encryptIdentity(
  passphrase: string,
  identity: LocalVaultIdentity,
): Promise<SealedVault> {
  const salt = randomBytes(VAULT_SALT_BYTES);
  const iv = randomBytes(VAULT_IV_BYTES);
  const key = await deriveVaultKey(passphrase, salt);
  const plaintext = new TextEncoder().encode(JSON.stringify(identity));
  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: arrayBufferFromBytes(iv) },
    key,
    arrayBufferFromBytes(plaintext),
  );

  return {
    version: VAULT_VERSION,
    kdf: VAULT_KDF,
    iterations: VAULT_ITERATIONS,
    saltHex: bytesToHex(salt),
    ivHex: bytesToHex(iv),
    ciphertextHex: bytesToHex(new Uint8Array(ciphertext)),
  };
}

async function decryptIdentity(passphrase: string, sealed: SealedVault): Promise<LocalVaultIdentity> {
  const salt = hexToBytes(sealed.saltHex);
  const iv = hexToBytes(sealed.ivHex);
  const ciphertext = hexToBytes(sealed.ciphertextHex);
  const key = await deriveVaultKey(passphrase, salt);
  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: arrayBufferFromBytes(iv) },
    key,
    arrayBufferFromBytes(ciphertext),
  );
  const identity = JSON.parse(new TextDecoder().decode(plaintext)) as LocalVaultIdentity;

  if (
    !identity.did.startsWith('did:exo:')
    || !identity.displayName
    || !/^[0-9a-f]{64}$/iu.test(identity.ed25519PublicHex)
    || !/^[0-9a-f]+$/iu.test(identity.ed25519PrivatePkcs8Hex)
    || !/^[0-9a-f]{64}$/iu.test(identity.x25519PublicHex)
    || !/^[0-9a-f]{64}$/iu.test(identity.x25519SecretHex)
  ) {
    throw new Error('Local VitalLock vault identity is invalid.');
  }

  return identity;
}

export async function createLocalVault(
  passphrase: string,
  displayName: string,
): Promise<LocalVaultIdentity> {
  const didRandomHex = bytesToHex(randomBytes(DID_RANDOM_BYTES));
  const x25519Keypair = generateX25519Keypair();
  const ed25519Keypair = await generateEd25519SigningMaterial();
  const identity: LocalVaultIdentity = {
    did: `did:exo:${didRandomHex}`,
    displayName: defaultDisplayName(didRandomHex, displayName),
    ed25519PublicHex: ed25519Keypair.publicKeyHex,
    ed25519PrivatePkcs8Hex: ed25519Keypair.privatePkcs8Hex,
    x25519PublicHex: x25519Keypair.public_key_hex,
    x25519SecretHex: x25519Keypair.secret_key_hex,
  };
  sealToStorage(await encryptIdentity(passphrase, identity));
  return identity;
}

export async function openLocalVault(passphrase: string): Promise<LocalVaultIdentity> {
  try {
    return await decryptIdentity(passphrase, loadSealedVault());
  } catch (err) {
    if (err instanceof Error && err.message.startsWith('No local VitalLock vault')) {
      throw err;
    }
    throw new Error('Unable to unlock local VitalLock vault with this passphrase.');
  }
}
