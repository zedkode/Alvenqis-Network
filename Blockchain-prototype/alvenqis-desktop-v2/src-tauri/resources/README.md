# Packaged runtime resources

Populated by `npm run prepare:native:sidecars` from `alvenqis-desktop/installer/stage`.

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
