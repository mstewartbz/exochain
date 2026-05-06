# Mission Economics

Mission economics are the EXOCHAIN core accounting model for purpose-bound work.

## Core Pattern

- `Mission` defines the economic container.
- `MissionPurpose` defines the problem, served party, promised outcome, risk surface, proof, and success condition.
- `ContributionReceipt` records useful work inside a Mission or contribution workflow.
- `HonorGoodRuleset` defines share lines per settlement basis.
- `MissionSettlement` computes settlement lines with checked integer arithmetic.

Settlement authority remains in EXOCHAIN core. CommandBase can show Mission state. ExoForge can propose receipts or rulesets. Neither simulates authoritative settlement locally.

## Accounting Rules

- No floats.
- Basis points only for fractional allocation.
- Each settlement basis is validated independently.
- Basis totals must not exceed 10,000 basis points.
- Overflow and underflow fail closed.
- Unsupported basis values fail closed.
- Zero amounts require explicit `ZeroFeeReason`.
- Payment, fiat, token, exchange, and external custody rails are outside this core accounting path.

## Apex Velocity Catalyst

Use explicit `ApexVelocityCatalyst` naming in code and docs where ambiguity exists. Bare `AVC` remains Autonomous Volition Credential.
