/**
 * Consent bailment builder.
 *
 * A bailment represents scoped, time-bounded consent from a bailor to a
 * bailee. `BailmentBuilder` mirrors the Rust SDK's builder pattern and
 * produces a {@link BailmentProposal} whose `proposalId` is a content-
 * addressed SHA-256 over the canonical fields.
 */

import { ConsentError } from '../errors.js';
import type { Did, Hash256 } from '../types.js';
import { validateDid } from '../identity/did.js';
import { sha256, bytesToHex } from '../crypto/hash.js';

/** A validated bailment proposal. */
export interface BailmentProposal {
  readonly proposalId: Hash256;
  readonly bailor: Did;
  readonly bailee: Did;
  readonly scope: string;
  readonly durationHours: number;
  readonly createdAt: number;
}

/** Builder for a {@link BailmentProposal}. */
export class BailmentBuilder {
  readonly #bailor: Did;
  readonly #bailee: Did;
  #scope?: string;
  #durationHours?: number;

  constructor(bailor: Did | string, bailee: Did | string) {
    this.#bailor = typeof bailor === 'string' ? validateDid(bailor) : bailor;
    this.#bailee = typeof bailee === 'string' ? validateDid(bailee) : bailee;
  }

  /** Set the scope string (e.g. `"data:medical"`). */
  public scope(scope: string): this {
    this.#scope = scope;
    return this;
  }

  /** Set the bailment duration in whole hours. */
  public durationHours(hours: number): this {
    this.#durationHours = hours;
    return this;
  }

  /** Validate the builder state and produce a {@link BailmentProposal}. */
  public async build(): Promise<BailmentProposal> {
    if (this.#scope === undefined) {
      throw new ConsentError('scope is required');
    }
    if (this.#scope.length === 0) {
      throw new ConsentError('scope must be non-empty');
    }
    if (this.#durationHours === undefined) {
      throw new ConsentError('durationHours is required');
    }
    if (!Number.isFinite(this.#durationHours) || this.#durationHours <= 0) {
      throw new ConsentError('durationHours must be > 0');
    }
    if (!Number.isInteger(this.#durationHours)) {
      throw new ConsentError('durationHours must be an integer');
    }

    const createdAt = Date.now();
    const proposalId = await computeProposalId(
      this.#bailor,
      this.#bailee,
      this.#scope,
      this.#durationHours,
    );

    return {
      proposalId,
      bailor: this.#bailor,
      bailee: this.#bailee,
      scope: this.#scope,
      durationHours: this.#durationHours,
      createdAt,
    };
  }
}

/**
 * Deterministic content-addressed proposal ID. Layout matches the Rust SDK's
 * canonicalization (fields joined with NUL separators, duration as LE u64),
 * but hashed with SHA-256 instead of BLAKE3.
 */
async function computeProposalId(
  bailor: Did,
  bailee: Did,
  scope: string,
  durationHours: number,
): Promise<Hash256> {
  const enc = new TextEncoder();
  const bailorB = enc.encode(bailor);
  const baileeB = enc.encode(bailee);
  const scopeB = enc.encode(scope);
  const durationB = new Uint8Array(8);
  // Little-endian u64 encoding.
  const view = new DataView(durationB.buffer);
  view.setBigUint64(0, BigInt(durationHours), true);

  const total =
    bailorB.length + 1 + baileeB.length + 1 + scopeB.length + 1 + durationB.length;
  const payload = new Uint8Array(total);
  let offset = 0;
  payload.set(bailorB, offset);
  offset += bailorB.length;
  payload[offset++] = 0;
  payload.set(baileeB, offset);
  offset += baileeB.length;
  payload[offset++] = 0;
  payload.set(scopeB, offset);
  offset += scopeB.length;
  payload[offset++] = 0;
  payload.set(durationB, offset);

  const digest = await sha256(payload);
  return bytesToHex(digest) as Hash256;
}
