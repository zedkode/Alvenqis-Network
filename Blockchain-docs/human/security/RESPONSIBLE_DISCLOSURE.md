# Responsible Security Disclosure

Status: Draft disclosure policy / no public bounty announced

Report a suspected vulnerability privately through the GitHub repository's
Security Advisory interface when available. Do not open a public issue for an
unpatched vulnerability, publish secrets, access unrelated data, disrupt public
services, or test against third-party infrastructure without authorization.

Include:

- affected immutable commit, component, and configuration;
- impact and realistic attack prerequisites;
- safe deterministic reproduction or proof of concept;
- logs with secrets and personal data removed;
- suggested mitigation if known;
- whether the issue may affect consensus, keys, funds-like test balances,
  availability, privacy, release integrity, or operator control.

The project must acknowledge, triage, remediate, retest, and coordinate
disclosure. Severity and validity are not decided by this document. No bug
bounty, audit pass, safe-harbor promise, or zero-vulnerability claim is created
until an explicit public program says so.
