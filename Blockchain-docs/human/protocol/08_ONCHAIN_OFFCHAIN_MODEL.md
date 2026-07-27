# On-Chain And Off-Chain Model

Status: Accepted data-boundary policy / future record types remain planned

## Fixed Rule

Large files, encrypted messages, images and payload-heavy communication remain off-chain.

The chain stores only critical settlement and proof data.

## On-Chain Data Categories

The accepted direction allows future explicitly implemented protocol features to store:
- ALVE transfers and settlement records;
- transaction fees;
- native asset records;
- NFT ownership records;
- NFT metadata hashes or URIs;
- software license proofs;
- Passport proof commitments;
- identity public keys;
- access rights;
- file hashes and storage proofs;
- encrypted channel registry records;
- message receipt hashes;
- marketplace settlement records;
- smart contract state.

## Off-Chain Data Categories

The following stay off-chain even when related product features are implemented:
- large files;
- NFT media files;
- game assets;
- encrypted message payloads;
- communication history;
- storage blobs and replicas;
- large metadata payloads;
- private profile data;
- media delivery assets.

## Design Principle

The protocol should commit to proofs, hashes, permissions and settlement, while storage and delivery layers handle large or private payloads off-chain.

## Impact Notes

- Core: state growth must be constrained by keeping payload-heavy content off-chain.
- Wallet: wallets may need proof display and linked off-chain data access later, but not direct payload storage.
- Explorer and Indexer: indexing should focus on commitments, proofs, ownership and events.
- RPC: upload or messaging payload APIs belong outside the core chain interface.
- Website/Admin and Docs: public messaging must emphasize proof-backed ownership, not on-chain file hosting.
