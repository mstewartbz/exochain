# Test Spec: CEO Onboarding Flow

## Description
E2E verification of the `ceo-onboarding.yaml` Syntaxis protocol. Simulates a first-time CEO check-in to the incubator.

## Test Steps
1. **Initialize**: Mock an `event: new_user_registration` trigger with a simulated CEO profile.
2. **Execute `welcome-message`**: Verify the system generates the correct welcome prompt. Provide a mock `welcome_read_receipt`.
3. **Execute `rapid-intake`**: Provide mock data (Company: Acme, Inventory: 2 Agents, Context: SOC2).
4. **Execute `team-activation`**: Trigger the node.
5. **Execute `first-task-assignment`**: Verify the Engagement Lead sends the ACRC prompt.

## Expected Outputs
- The DAG must record a `Draft` to `IdentityResolved` to `Executed` state transition.
- The `active_team_manifest` must contain 4 valid agent IDs.
- The dashboard UI configuration payload must be generated successfully.

## Pass Criteria
The protocol reaches the `Executed` state within 500ms without throwing an unhandled exception.

## Rollback Procedure
If the test fails, delete the mock user profile and purge the mock DAG entries using the test runner's teardown script.