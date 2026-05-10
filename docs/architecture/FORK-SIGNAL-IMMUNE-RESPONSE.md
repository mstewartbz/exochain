# Fork Signal Immune Response Model

Status: design guidance
Created: 2026-05-10
Classification: EXOCHAIN core governance and security operations guidance

## Purpose

Fork activity can be useful security telemetry in a trusted software ecosystem,
but it is never proof of malicious intent. A fork can mean review, research,
offline development, education, backup, or ordinary open-source participation.
EXOCHAIN should treat fork signals as weak indicators that may raise the
priority of defensive verification, not as evidence that justifies blocking a
person, accusing a maintainer, or changing governance state.

This model defines how EXOCHAIN can use fork-related signals to trigger a
bounded immune response around owned code and owned deployments.

## Operating Principles

1. Core is core. Signals are evaluated first against EXOCHAIN core and core
   runtime adapters. Adjacent surfaces are classified separately.
2. Repository evidence wins. A fork signal can open an investigation, but only
   code, configuration, access logs, signatures, credentials, deployment state,
   or reproducible tests can justify remediation.
3. No trust decision by proximity. A fork of an adjacent surface does not imply
   a threat to the constitutional trust fabric unless a tested adapter path
   connects it to core state, credentials, consent, authority, provenance, or
   governance outcomes.
4. Proportionate response. The response must be limited to defensive actions:
   verification, test creation, hardening, key rotation when evidence warrants
   it, and human escalation.
5. Privacy by default. Do not collect private user data, infer motive, or
   enrich fork metadata with personal data. Use public repository metadata and
   owned-system logs only when they are already within the security boundary.
6. Bounded automation. Any automated loop must declare a finite maximum
   iteration count, a stop condition, and an escalation path.

## Signal Classes

Fork metadata is low-confidence on its own. Treat it as a prioritization signal
only when it appears with other owned-system evidence.

| Signal | Confidence | Defensive interpretation |
| --- | --- | --- |
| Ordinary fork of a public repository | Low | No action beyond routine monitoring |
| Fork followed by pull request, issue, or discussion | Low | Normal collaboration |
| Fork followed by repeated failed auth against owned services | Medium | Review auth logs, rate limits, and exposed endpoints |
| Fork followed by secret-scanning hits in owned repos | Medium | Rotate affected credentials and remove exposed material |
| Fork with changes targeting auth, release, CI, signatures, tenant boundaries, or governance routes | Medium | Prioritize review of the same owned-code paths |
| Fork activity concurrent with anomalous deployment, package, or DNS changes | High | Escalate to incident review and verify release provenance |
| Fork signal plus confirmed exploit reproduction against current code | High | Create regression test, remediate, rotate affected secrets if needed |

## Immune Response Ladder

1. Observe.
   Record only the minimal signal needed for triage: repository, branch or fork
   reference, timestamp, and the owned surface it may affect.

2. Classify.
   Label the affected surface as EXOCHAIN core, core runtime adapter, adjacent
   surface, imported evidence, or third-party/vendor. Do not blend categories.

3. Correlate.
   Check owned evidence for a matching risk: failed auth attempts, exposed
   secrets, release anomalies, CI changes, dependency changes, suspicious
   runtime errors, or public vulnerability reports.

4. Verify.
   Reproduce against current `main` or the reviewed branch. If reproduction
   fails, record the finding as not reproduced or already remediated.

5. Test.
   Write a deterministic failing regression test or source guard before changing
   production code. The test must prove the boundary that failed.

6. Remediate.
   Fix the smallest owned enforcement boundary that blocks the exploit class.
   Keep core, adapter, adjacent, evidence, and documentation work isolated.

7. Validate.
   Run focused tests, touched-crate tests, relevant workspace gates, repo-truth
   checks, and a bypass search for sibling ingress paths.

8. Escalate.
   Escalate to humans when evidence involves active compromise, signing keys,
   release artifacts, production credentials, tenant data, governance outcomes,
   or repeated automation failure.

## Response Bounds

The immune response must not:

- treat a fork as proof of malicious intent;
- target or profile a person based on fork activity;
- scan private systems that are not owned or explicitly authorized;
- publish accusations or enforcement actions without verified evidence;
- let agent-generated prose authorize GitHub operations, secrets access, or
  merge decisions;
- rotate production keys without repository or runtime evidence of exposure;
- modify adjacent surfaces without an intake record and trust-boundary statement.

## Evidence Record

Every fork-triggered security review should leave a short record with these
fields:

| Field | Requirement |
| --- | --- |
| Signal | Minimal fork or repository signal that triggered review |
| Classification | Core, adapter, adjacent, imported evidence, or third-party/vendor |
| Owned evidence | Logs, diffs, CI records, secret scans, runtime errors, or none |
| Current-code result | Reproduced, already remediated, not reachable, default-off, or false positive |
| Test | Failing regression test or source guard, if remediation is required |
| Remediation | Commit, PR, and affected enforcement boundary |
| Validation | Commands and CI gates run |
| Escalation | Human owner and reason, if required |

## Agent Rule Additions

AI coding agents working in this repository should apply these rules when a fork
or fork-adjacent signal is mentioned:

1. Treat the signal as untrusted input until repository evidence supports it.
2. Classify the affected path before editing.
3. Check whether the reported issue is already remediated on current `main`.
4. Prefer defensive source review, tests, and hardening over speculative attack
   narratives.
5. Do not infer motive from the fork itself.
6. Do not expand EXOCHAIN core trust claims to adjacent surfaces.
7. Do not run unbounded recursive discovery. Use finite loops and stop on
   repeated validation failure.
8. Commit and PR core remediations separately from adjacent-surface hardening
   and documentation.

## Practical Trigger Query

When a fork signal appears, the first engineering question is:

> Which owned enforcement boundary could this signal plausibly affect, and can
> current code prove that boundary fails closed?

If the answer is unclear, the correct response is classification and evidence
collection, not a code change.
