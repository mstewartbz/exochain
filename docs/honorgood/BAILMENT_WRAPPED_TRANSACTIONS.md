# Bailment-Wrapped Transactions

A bailment-wrapped transaction links contribution adoption, use, value measurement, and settlement to accepted terms.

## Flow

1. A `ValueContributionNode` is offered.
2. A `ContributionOffer` binds terms, permitted use, prohibited use, adoption policy, and settlement ruleset.
3. A `ContributionAcceptance` records accepted terms and delegated authority.
4. A `BailmentWrapper` binds the contribution, offer, acceptance, terms, custody scope, and authority references.
5. `AdoptionEvent`, `UseEvent`, and `ValueEvent` establish use and measurable value.
6. `AutomatedSettlementEvent` may execute only if all fail-closed checks pass.

## Fail-Closed Conditions

Automated settlement is rejected if any required offer, acceptance, wrapper, authority proof, ruleset, value event, legal effect, or materiality proof is missing or invalid. It is also rejected when a dispute, revocation, suspension, high-risk custody exception, or human-approval requirement is active.
