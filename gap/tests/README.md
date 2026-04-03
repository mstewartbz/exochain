# GAP Test Suite

This directory contains the End-to-End (E2E) test specifications for the `gap/` incubator layer. 

## Overview
These tests do not verify the underlying ExoChain crates; they verify the *orchestration* of those crates via the Syntaxis protocols defined in `gap/syntaxis/`. They ensure that the governance pipelines, agentic handoffs, and Decision Forum routing function flawlessly.

## How to Run
These test specs are machine-readable by the ExoForge Liaison during the CQI self-improvement loop. Before any system patch is deployed, the Liaison must execute the automated test runner against these specs.
```bash
# Example execution (simulated)
exo-test run ./gap/tests/
```

## CI Integration
These tests run automatically on every PR submitted to the `exochain` repository that modifies anything within the `gap/` directory.

## Coverage Requirements
The `gap/` layer requires 100% pass rate on all E2E tests and strict validation against `test-invariant-coverage.md`. A single failure constitutes a broken build and triggers a mandatory rollback.