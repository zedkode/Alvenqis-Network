# Alvenqis Brand Migration

Status: Mainnet Candidate compatibility policy

The product name is **Alvenqis**. New package names, executables, environment
variables, UI copy and runtime directories use `Alvenqis`, `alvenqis` and `ALVENQIS`.

## Compatibility identifiers

Some serialized identifiers retain the historical `alvenqis` spelling. They are
protocol or persistence values, not product branding:

- network IDs: `alvenqis-devnet`, `alvenqis-testnet`, `alvenqis-mainnet-candidate`;
- transaction signing domain: `alvenqis-tx-ed25519-v1`;
- wallet schema IDs already written to disk;
- genesis review and approval standard IDs;
- the `Alvenqis Mainnet Candidate` human-name field inside the already hashed
  genesis review and approval records;
- published wire-test-vector payloads.

Changing any of these values without a separately approved protocol migration
would split the network, change transaction signatures, invalidate genesis
evidence or make existing wallet data unreadable.

## Non-destructive runtime migration

- Desktop startup copies missing Alvenqis Control Center files into the Alvenqis
  profile and leaves the legacy profile intact.
- The keystore helper reads a missing Alvenqis credential from the legacy Alvenqis
  service, writes an equivalent Alvenqis credential and retains the old entry.
- VPS repair copies legacy chain/control/pool data into Docker-managed state.
  Legacy services and conflicting containers are stopped and disabled or
  renamed for rollback; they are not deleted.

The active Docker deployment lives only in
`alvenqis-release/vps-control-plane/`. The `alvenqis-docker` staging folder was an
import source and must not become a parallel control-plane implementation.
