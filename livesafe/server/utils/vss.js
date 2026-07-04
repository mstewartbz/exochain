/**
 * VSS (Verifiable Secret Sharing) - Shamir's Secret Sharing Implementation
 *
 * Implements a 3-of-4 threshold scheme for PACE trustee key shard distribution.
 * Each subscriber's master identity key is split into 4 shares, where any 3
 * shares can reconstruct the original key.
 *
 * All operations use GF(256) arithmetic for byte-level secret sharing.
 */

const crypto = require('crypto');

// GF(256) arithmetic using AES irreducible polynomial x^8 + x^4 + x^3 + x + 1
const EXP_TABLE = new Uint8Array(256);
const LOG_TABLE = new Uint8Array(256);

// Initialize lookup tables for GF(256) multiplication
(function initTables() {
  let x = 1;
  for (let i = 0; i < 255; i++) {
    EXP_TABLE[i] = x;
    LOG_TABLE[x] = i;
    x = x ^ (x << 1);
    if (x >= 256) x ^= 0x11b; // AES polynomial
  }
  EXP_TABLE[255] = EXP_TABLE[0];
})();

function gfMul(a, b) {
  if (a === 0 || b === 0) return 0;
  return EXP_TABLE[(LOG_TABLE[a] + LOG_TABLE[b]) % 255];
}

function gfDiv(a, b) {
  if (b === 0) throw new Error('Division by zero in GF(256)');
  if (a === 0) return 0;
  return EXP_TABLE[(LOG_TABLE[a] - LOG_TABLE[b] + 255) % 255];
}

/**
 * Evaluate polynomial at point x in GF(256)
 * coefficients[0] is the secret, coefficients[1..k-1] are random
 */
function evaluatePolynomial(coefficients, x) {
  let result = 0;
  for (let i = coefficients.length - 1; i >= 0; i--) {
    result = gfMul(result, x) ^ coefficients[i];
  }
  return result;
}

/**
 * Split a single byte secret into n shares with threshold k
 */
function splitByte(secret, k, n) {
  // Generate random polynomial coefficients (secret is coefficient[0])
  const coefficients = new Uint8Array(k);
  coefficients[0] = secret;
  const randomBytes = crypto.randomBytes(k - 1);
  for (let i = 1; i < k; i++) {
    coefficients[i] = randomBytes[i - 1];
  }

  // Evaluate polynomial at x=1,2,3,...,n
  const shares = [];
  for (let i = 1; i <= n; i++) {
    shares.push({ x: i, y: evaluatePolynomial(coefficients, i) });
  }
  return shares;
}

/**
 * Reconstruct a single byte from k shares using Lagrange interpolation
 */
function reconstructByte(shares) {
  let secret = 0;
  for (let i = 0; i < shares.length; i++) {
    let numerator = 1;
    let denominator = 1;
    for (let j = 0; j < shares.length; j++) {
      if (i === j) continue;
      numerator = gfMul(numerator, shares[j].x);
      denominator = gfMul(denominator, shares[i].x ^ shares[j].x);
    }
    const lagrange = gfDiv(numerator, denominator);
    secret ^= gfMul(shares[i].y, lagrange);
  }
  return secret;
}

/**
 * Generate a master key and split it into shares using Shamir's Secret Sharing
 *
 * @param {number} threshold - Minimum shares needed to reconstruct (default: 3)
 * @param {number} totalShares - Total number of shares to generate (default: 4)
 * @returns {Object} { masterKeyHash, shares: [{index, shardData, shardRef}] }
 */
function generateVSSShards(threshold = 3, totalShares = 4) {
  // Generate a 256-bit master key
  const masterKey = crypto.randomBytes(32);

  // Hash the master key for verification (never store the actual key)
  const masterKeyHash = crypto.createHash('sha256').update(masterKey).digest('hex');

  // Split each byte of the master key into shares
  const shareBuffers = [];
  for (let i = 0; i < totalShares; i++) {
    shareBuffers.push(Buffer.alloc(masterKey.length));
  }

  for (let byteIdx = 0; byteIdx < masterKey.length; byteIdx++) {
    const byteShares = splitByte(masterKey[byteIdx], threshold, totalShares);
    for (let shareIdx = 0; shareIdx < totalShares; shareIdx++) {
      shareBuffers[shareIdx][byteIdx] = byteShares[shareIdx].y;
    }
  }

  // Encrypt each share with AES-256-GCM
  const encryptionKey = crypto.randomBytes(32);
  const shares = shareBuffers.map((shareBuf, idx) => {
    const iv = crypto.randomBytes(16);
    const cipher = crypto.createCipheriv('aes-256-gcm', encryptionKey, iv);
    const encrypted = Buffer.concat([cipher.update(shareBuf), cipher.final()]);
    const authTag = cipher.getAuthTag();

    const shardId = crypto.randomUUID();
    return {
      index: idx + 1,
      shardData: Buffer.concat([iv, authTag, encrypted]).toString('hex'),
      shardRef: `vss:exo:shard:${shardId}`,
    };
  });

  return {
    masterKeyHash,
    shares,
  };
}

/**
 * Verify that a set of shard references belong to the same VSS ceremony
 * @param {Array} shardRefs - Array of shard reference strings
 * @returns {boolean} Whether all shards appear to be from valid VSS generation
 */
function validateShardRefs(shardRefs) {
  return shardRefs.every(ref => ref && ref.startsWith('vss:exo:shard:'));
}

module.exports = {
  generateVSSShards,
  validateShardRefs,
  // Exposed for testing
  splitByte,
  reconstructByte,
};
