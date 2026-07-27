# alvenqis-explorer

Status: Draft / Mainnet Candidate / Prototype / Not Live Mainnet

This app provides a read-only Mainnet Candidate explorer UI served exclusively by `alvenqis-rpc-gateway`.

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
