# Defensive Security Engineering Scope

Status: Authorized project-owned defensive engineering policy

## Purpose

Security work in this repository exists to prevent failures, validate safeguards,
improve input handling, reduce operational risk, and document remediation for
Alvenqis systems. It is not authorization to access, disrupt, or test unrelated
systems.

This policy applies to agent instructions, task descriptions, reviews, tests,
reports, and reproducer artifacts.

## Authorization boundary

Work is limited to:

- source code, configuration, tests, and documentation in this repository;
- local test processes and disposable local data created for the task;
- project-owned Devnet or explicitly identified project staging systems when the
  owner separately authorizes active testing of those systems.

Public endpoints may receive ordinary functional probes only when a task explicitly
requires them. Load, fault-injection, or adversarial traffic must run locally or in
an explicitly authorized isolated environment. Ownership or scope uncertainty is a
stop condition.

The following are outside this standing authorization:

- third-party systems, accounts, networks, or data;
- credential acquisition, phishing, social engineering, or bypassing access
  controls;
- stealth, persistence, malware, destructive actions, or denial-of-service traffic;
- deployment, live configuration changes, or disclosure of unresolved sensitive
  details unless the owner explicitly authorizes that separate action.

## Defensive methods

Permitted project methods include:

- static code and configuration review;
- dependency, secret, and supply-chain hygiene checks;
- local property-based tests and bounded fuzzing of parsers;
- unit, integration, interoperability, and recovery tests;
- threat modeling focused on current safeguards and planned improvements;
- bounded local or isolated-environment resilience and load testing;
- minimal non-destructive reproducers needed to prove and fix a repository defect.

Tests must prefer synthetic data, preserve service availability, and stop on
unexpected external impact.

## Reporting requirements

Reports must state:

- the owned component and environment that were tested;
- the exact defensive objective and bounded method;
- commands, duration, and relevant evidence;
- findings with severity, affected safeguard, and remediation;
- checks that were not run and any authorization needed to run them.

Do not include real secrets or operational instructions that would unnecessarily
increase risk while a finding remains unresolved. Use the private responsible
disclosure process for sensitive details.

