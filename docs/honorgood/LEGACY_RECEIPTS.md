# Legacy Receipts

`LegacyReceipt` is the HonorGood object for evergreen provenance and conditional participation. It records who or what contributed, what downstream system uses it, why the contribution is material, what attribution is required, and whether economic terms are merely proposed or have legal effect.

## State Machine

Allowed state movement is formal and fail-closed:

- `Proposed -> Recognized`
- `Proposed -> Offered`
- `Proposed -> Rejected`
- `Proposed -> Superseded`
- `Recognized -> Offered`
- `Recognized -> Rejected`
- `Recognized -> Deprecated`
- `Recognized -> Superseded`
- `Offered -> ContributorAccepted`
- `Offered -> Rejected`
- `Offered -> Deprecated`
- `Offered -> Superseded`
- `ContributorAccepted -> Ratified`
- `ContributorAccepted -> Rejected`
- `ContributorAccepted -> Superseded`
- `Ratified -> Deprecated`
- `Ratified -> Superseded`

Direct `Proposed -> Ratified` is rejected. Ratification requires a signed contributor acceptance hash, a human ratifier DID, and `RatifiedAgreement` legal effect.

## Materiality Review

`Genesis`, `Foundational`, and `Material` classifications must carry review metadata:

- reviewer DID
- evidence hash
- rationale hash
- optional rationale reference
- HLC reviewed-at timestamp
- review status

Materiality is not valid merely because it is asserted. A disputed materiality review blocks automated settlement.

## Beneficiary References

Beneficiaries are opaque. Valid references are DIDs, public project treasuries, vault pointers, or hashed references. Sensitive personal, banking, tax, family, estate, and payment data must remain off-ledger.

## Seed Receipts

The repository includes seed examples for:

- Archon as proposed upstream provenance for ExoForge.
- Paperclip as proposed upstream provenance for CommandBase.

Both are recognition proposals. Neither is ratified. Neither creates a present legal obligation.
