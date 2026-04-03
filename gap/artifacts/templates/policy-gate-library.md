# Template: Policy Gate Library

*Instructions for Artifact Builder: Translate human business rules into exact, executable conditional logic.*

## 1. Gate Categories
*Group gates by their primary function.*
- **Financial Constraints**
- **Data Privacy (PII) Checks**
- **Operational Rate Limits**

## 2. Gate Definitions
*Format: IF [condition] THEN [action] ELSE [fallback]*

### Gate: [Gate ID, e.g., FIN-01]
- **Description**: [Human readable rule, e.g., Prevent single transactions over $10,000]
- **Logic**: 
  ```yaml
  if: transaction.amount > 10000
  then: route_to_escalation(human_controller)
  else: approve_and_anchor
  ```
- **NemoClaw Deployment Notes**: [e.g., Requires integer parsing of the amount field before evaluation. Ensure currency conversion is applied.]

### Gate: [Gate ID, e.g., PRIV-01]
- **Description**: [Human readable rule, e.g., Scan outputs for SSNs before sending external email]
- **Logic**: 
  ```yaml
  if: regex_match(output_body, SSN_PATTERN) == true
  then: block_execution AND flag_security
  else: proceed_to_send
  ```
- **NemoClaw Deployment Notes**: [e.g., Use standard regex library; alert must include the agent ID attempting the send.]

---
**Approval Block**
- Prepared by: `artifact-builder`
- Council Status: `[Draft / Governed]`
- CEO Signature: `[Pending]`