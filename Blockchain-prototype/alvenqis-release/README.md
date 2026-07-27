# Alvenqis Release

Release process assets, candidate manifests, checksums and deployment bundles.

Status: Mainnet Candidate. This folder does not declare a live public mainnet.

- `vps-control-plane/` is the only active Ubuntu VPS distribution.
- `vps/` is legacy/frozen and must not receive product or security changes.
- `apps/` is the ignored local destination for generated Windows, Linux and
  Android packages. Only its documentation and release manifests are tracked.
- GitHub candidate releases are prereleases until native signing, external
  security review and the release checklist are complete.
