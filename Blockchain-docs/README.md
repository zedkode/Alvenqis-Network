# Alvenqis Documentation

Status: Current documentation boundary / mixed implementation maturity

`Blockchain-docs/human/` is the public, version-controlled documentation source.
It contains protocol, architecture, API, operations, release, security, and
audit-preparation material. The public source-information set lives under
`Blockchain-docs/human/source-info/`.

`Blockchain-docs/internal/` is a local, private workspace containing the
canonical task tracker and decision registers. It is intentionally excluded from
the public repository. Other private gate directives and review workspaces must
never be copied into public documentation.

Public entry points:

- [`../README.md`](../README.md)
- [`../PROJECT_STRUCTURE.md`](../PROJECT_STRUCTURE.md)
- [`../init.md`](../init.md)
- [`human/README.md`](./human/README.md)
- [`human/release/NETWORK_MATURITY.md`](./human/release/NETWORK_MATURITY.md)
- [`human/security/README.md`](./human/security/README.md)

Validate documentation with:

```bash
node Blockchain-scripts/docs/audit-docs.mjs
```
