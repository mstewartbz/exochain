# Apex Velocity Catalyst Mission Economics Compact

Status: canonical internal compact for Apex Velocity Catalyst mission economics.

Important naming: `exo-avc` means Autonomous Volition Credential. This compact uses Apex Velocity Catalyst explicitly and does not redefine Autonomous Volition Credential behavior.

## Doctrine

Mission creates context.
Purpose creates alignment.
Receipts create proof.
Rulesets create fairness.
Settlement creates trust.
EXOCHAIN records the whole thing.

Membership creates access.
Contribution creates receipts.
Receipts create settlement.
EXOCHAIN creates trust.

## Scope

Apex Velocity Catalyst mission economics are represented by EXOCHAIN core objects:

- `Mission`
- `MissionPurpose`
- `ContributionReceipt`
- `HonorGoodRuleset`
- `SettlementLine`
- `MissionSettlement`
- `ValueContributionNode`
- `ContributionOffer`
- `ContributionAcceptance`
- `BailmentWrapper`
- `AdoptionEvent`
- `UseEvent`
- `ValueEvent`
- `AutomatedSettlementEvent`

Rulesets are templates and fixtures, not hardcoded universal law. Settlement uses checked integer arithmetic and basis points.

## Default Templates

Client services example:

- 15% Apex Velocity Catalyst / EXOCHAIN protocol fee.
- 10% originator.
- 5% closer or deal architect.
- 60% delivery budget.
- 10% mission surplus or outcome pool.

Software/channel example:

- 80% to 90% platform company retained revenue.
- 10% to 20% Apex Velocity Catalyst channel/adoption fee.
- Channel fee split: 40% originator, 40% implementation/governance contributors, 20% protocol margin.

These are recorded as ruleset examples. They do not execute unless a Mission, accepted terms, valid receipts, value event, and settlement record support them.

## Zero-Launch Compatibility

Objects, receipts, rulesets, settlement lines, and settlement hashes may exist while all amounts resolve to zero. Any zero settlement must carry an explicit `ZeroFeeReason`. The compact does not add fiat rails, token rails, external exchanges, custody rails, or automated payment execution.
