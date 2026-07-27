# alvenqis-explorer

Status: Mainnet Candidate / Read-only public explorer

This app provides a read-only Mainnet Candidate explorer UI served exclusively by `alvenqis-rpc-gateway`.

## Production container

Build directly from this directory:

```bash
docker build -t alvenqis-explorer .
docker run --rm -p 8080:8080 --read-only --tmpfs /tmp:rw,noexec,nosuid,size=16m alvenqis-explorer
```

The Nginx runtime is unprivileged, serves the SPA on port `8080`, writes runtime files only below `/tmp`, and exposes `GET /healthz`.

Compose service:

```yaml
alvenqis-explorer:
  build:
    context: ./Blockchain-prototype/alvenqis-explorer
  read_only: true
  tmpfs:
    - /tmp:rw,noexec,nosuid,size=16m
  ports:
    - "8080:8080"
```

Current scope:
- dashboard, latest blocks, block details, transaction details, address details and network status;
- local mempool visibility and latest mined transaction visibility;
- explicit network badges from RPC metadata;
- environment config through `VITE_ALVENQIS_RPC_URL`.

Candidate startup:
1. Start the node with `configs/mainnet-candidate.toml`.
2. Run the indexer with `--network mainnet-candidate index-chain`.
3. Start `alvenqis-rpc-gateway` with `configs/rpc.mainnet-candidate.toml`.
4. In this folder run `npm install` and `npm run dev`.

Default example env:
- `VITE_ALVENQIS_RPC_URL=http://127.0.0.1:10787`

Important limitations:
- no public deployment;
- no wallet connect;
- no send transaction from the UI;
- no separate public test network and no live mainnet claim until launch gates pass.
