# Template: Authority Matrix

*Instructions for Artifact Builder: Define exactly who holds what power. Leave no ambiguity regarding cryptographic delegations.*

## 1. Role Definitions
*Define the actors (human and agent) participating in the workflows.*

- **Role A**: [e.g., Senior Financial Controller (Human)]
- **Role B**: [e.g., Reconciliation Agent V2 (Agent)]
- **Role C**: [e.g., Operations Panel (Council)]

## 2. Permission Levels
*Define the distinct tiers of access and execution capability.*

- **Level 1 (Read-Only)**: Can view DAG logs and current state.
- **Level 2 (Propose)**: Can draft state changes and submit to BCTS `Submitted` state.
- **Level 3 (Execute-Low)**: Can execute T0 operations without human review.
- **Level 4 (Execute-High)**: Can execute T1+ operations (Requires Dual Control).
- **Level 5 (Admin/Override)**: Can trigger system halts or executive overrides.

## 3. Delegation Chains
*Map Roles to Permissions and define constraints.*

| Actor Role | Permission Level | Resource Scope | Constraints |
| :--- | :--- | :--- | :--- |
| [Example: Recon Agent] | [Level 2] | [Ledger API] | [Cannot exceed $5k per tx] |
| [Example: Controller] | [Level 4] | [Ledger API] | [Requires MFA] |

## 4. Revocation Triggers
*Define exactly what conditions automatically revoke an agent's authority.*
- [e.g., 3 consecutive policy gate failures]
- [e.g., Confidence score drops below 85%]

---
**Approval Block**
- Prepared by: `artifact-builder`
- Council Status: `[Draft / Governed]`
- CEO Signature: `[Pending]`