# Seed Diversity and Peer Discovery

Status: Draft G2 operator contract / current candidate default non-compliant

The current Mainnet Candidate config names one unpinned project DNS seed and
does not enable active Kademlia/PEX-style discovery. This document defines the
evidence required to replace that bootstrap single point.

## Required seed set

- three to five independently controlled operators;
- PeerId-pinned multiaddresses;
- multiple hosting providers, networks/ASNs, jurisdictions, and DNS control
  planes;
- no shared controller credential or mandatory project tunnel;
- published change/retirement process;
- continuous reachability monitoring that does not grant consensus privilege.

## Node requirements

- reject a configured seed when the resolved peer does not match the pinned
  PeerId;
- rotate and retry seeds with bounded backoff;
- reserve outbound capacity;
- discover peers beyond the seed set;
- bound discovery storage, dial concurrency, and inbound sources;
- apply IP/subnet/ASN admission in addition to PeerId reputation;
- keep the network usable after every project-operated seed is removed.

## G2 evidence

1. immutable seed manifest with operator and PeerId attestations;
2. clean nodes bootstrapped through different independent seeds;
3. DNS poisoning and wrong-PeerId negative tests;
4. seed outage and active-discovery failover;
5. Sybil attempts from one IP/subnet and many PeerIds;
6. partition/reconnect and resume transcript;
7. proof that seed operators cannot bypass validation or force fork choice.

No currently configured seed receives validator, checkpoint, upgrade, mining,
or governance authority.
