# Packaged runtime resources

Populated by `npm run prepare:native:sidecars` from the monorepo binaries,
configs, release documents, explorer build, and operator scripts.

Expected layout after staging:

```
resources/
  bin/                 alvenqis-node, miner, rpc-gateway, indexer, keystore-helper
  scripts/local/       operator helpers
  configs/             genesis + local configs
  docs/release/        genesis review artifacts
  explorer/            static explorer
  alvenqis.ps1 / .cmd    operator entrypoints (Windows stage)
```

Development builds do not require this folder; the monorepo root is used as the workspace.
