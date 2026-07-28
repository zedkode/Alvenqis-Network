# ALVENQIS NETWORK - RAPORT DE AUDIT TEHNIC

**Clasificare:** audit intern, nerecenzat independent  
**Data:** 2026-07-28  
**Repository:** `https://github.com/zedkode/Alvenqis-Network`  
**Snapshot final auditat:** `1a765acc75fff7029bc209429b232b7b92fb9a34`  
**Sursă de adevăr pentru ordine și scope:** `reform.md`, Faza 1, Task 1.1-1.5  
**Documentație de evidență:** `D:\Blockchain-Core\ALVENQIS_MAINET_DOCUMENTATION.md`

## 1. Verdict executiv

```text
MAINNET READY: NU
SECURITY AUDIT PASSED: NU
VULNERABILITĂȚI/PROBLEME DESCHISE: DA
P2P ECLIPSE RESISTANT: NU
RPC STABLE: NU
VPS REDEPLOY AUTORIZAT ÎN FAZA 1: NU
```

Proiectul are controale tehnice reale: transport P2P Noise autentificat, handshake de rețea/genesis, validare completă a blocurilor înainte de adoptare, profile RPC, body limits, validare de tranzacții și template-uri mining limitate în memorie.

Aceste controale nu sunt suficiente pentru mainnet. Auditul confirmă:

- advisory-uri Rust blocante și un advisory `unsound` în Desktop Linux;
- bootstrap P2P unic, neancorat la PeerId și fără discovery activ;
- protecție Sybil/eclipsă insuficientă;
- lipsa rate limiting-ului pentru read RPC și bypass al rate limiting-ului prin forwarding headers;
- profil RPC live contradictoriu: mining public, dar transmiterea tranzacțiilor este 404;
- endpoint public degradat, zero peers și indexer cu 215 blocuri lag la momentul verificării;
- build/security gates care nu pot produce o atestare „0 issues”.

Nu a fost confirmat un bypass direct de consens în fluxurile P2P/RPC analizate. Aceasta nu reprezintă o garanție criptografică sau un penetration test complet.

## 2. Rezumat severitate

| SEVERITATE | NUMĂR | INTERPRETARE |
|---|---:|---|
| Critical | 0 | niciun exploit critic confirmat în scope-ul Fazei 1 |
| High | 17 | remediere obligatorie înainte de mainnet/public release |
| Medium | 18 | hardening și corectitudine obligatorii înainte de readiness |
| Low | 6 | debt tehnic și defense-in-depth |
| **TOTAL** | **41** | finding-uri unice, fără dublarea acelorași cauze |

Absența finding-urilor Critical nu înseamnă că proiectul este sigur. Auditul nu a inclus fuzzing complet, stres extern, multi-ASN eclipse simulation, audit criptografic independent sau penetration testing pe VPS.

## 3. Scope

### 3.1 Cod și configurație

- 11 pachete Rust în workspace-ul principal;
- workspace Tauri Desktop V2;
- workspace keystore-helper;
- 158 fișiere Rust, 44.229 linii negoale;
- configurații node, RPC, pool, VPS control-plane;
- P2P, peer reputation, storage și route/service layers;
- CI/release/install observations identificate în Task 1.1.

### 3.2 Runtime verificat

Au fost făcute cereri neintruzive către:

```text
https://rpcnode.dohotstudio.com
```

Nu au fost trimise tranzacții valide, blocuri, share-uri sau mining template requests cu adresă validă.

### 3.3 Exclus explicit

- modificări de consens sau fork-choice;
- migrarea storage;
- ștergeri/redeploy pe VPS;
- orice modificare a `1Panel`, n8n, vaultwarden, gitea sau uptime-kuma;
- audit extern independent;
- simulări DDoS/eclipsă pe infrastructura publică;
- semnare PGP a raportului.

## 4. Metodologie și dovezi

### 4.1 Toolchain

```text
rustc 1.97.0
cargo 1.97.0
clippy 0.1.97
cargo-audit 0.22.2
cargo-deny 0.20.2
cargo-outdated 0.19.0
cargo-geiger 0.13.0
```

### 4.2 Comenzi principale

```powershell
cargo check --workspace --all-targets
cargo audit
cargo deny check
cargo outdated
cargo geiger
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
cargo test -p alvenqis-node --lib p2p::tests
cargo test -p alvenqis-node --lib peer_reputation::tests
cargo test -p alvenqis-node --test devnet p2p_handshake
cargo test -p alvenqis-rpc-gateway --test rpc
```

### 4.3 Rezultate gate-uri

| CONTROL | REZULTAT |
|---|---|
| `cargo check --workspace --all-targets` | PASS pe baseline-ul auditat |
| `cargo audit` principal | FAIL: 2 vulnerabilități, 2 warnings |
| `cargo audit` Desktop | PASS fără vulnerability, dar 17 warnings |
| `cargo audit` helper | FAIL: 2 vulnerabilități moștenite |
| `cargo deny check` principal | FAIL |
| `cargo deny check` Desktop | FAIL: `glib` unsound |
| `cargo deny check` helper | FAIL |
| Clippy strict | FAIL, exit 101 |
| `cargo outdated` exact | PARTIAL: blocat de migrarea feature-ului Reqwest |
| Geiger | complet pe 13 pachete, cu limitările tool-ului documentate |
| P2P tests | PASS: 11 + 10 + 2 |
| RPC integration tests | PASS: 21 |

## 5. Puncte forte confirmate

- Noise autentifică transportul P2P și Yamux limitează stream-urile;
- handshake-ul aplicației validează protocol version, network ID, chain magic și genesis;
- Gossipsub folosește mesaje semnate și validare strictă de identitate;
- blocurile sunt validate prin node/core înainte de persistență sau reorg;
- reputația P2P persistă și permite ban/unban operator;
- RPC are profile explicite și ascunde rute mutante pe profile read-only;
- request body are limită implicită de 64 KiB;
- erorile 500 sunt redactate;
- mining template IDs sunt criptografic random, expiră și sunt capped;
- pool admin folosește Bearer token, `ct_eq` și expirarea tokenului;
- indexer pagination este capped pentru rutele paginate.

## 6. Finding-uri High

| ID | DESCRIERE | IMPACT | REPRODUCERE / DOVADĂ | REMEDIERE |
|---|---|---|---|---|
| `SUP-H01` | `RUSTSEC-2026-0119` în `hickory-proto 0.25.2` | DNS response poate provoca CPU O(n²) | `cargo audit`; lanț `libp2p-dns 0.44.0 -> hickory 0.25.2` | eliminare DNS/IP-only coordonată sau primul release libp2p cu Hickory 0.26.1+ |
| `SUP-H02` | `glib 0.18.5` este marcat `unsound` pentru Desktop Linux | risc memory-safety în stack-ul GTK/Tauri | `cargo deny check` în workspace Desktop | upgrade Tauri/GTK la un graph fără advisory și build Linux complet |
| `QLT-H01` | security/build gates obligatorii eșuează | nu poate exista atestare reproductibilă de release sigur | audit, deny și Clippy strict returnează non-zero | gate CI fail-closed pentru toate workspace-urile, cu excepții documentate și expirabile |
| `REL-H01` | release gates au tag mismatch și nu acoperă Tauri/helper | artefacte pot fi omise sau publicate fără aceleași controale | workflow `desktop-v*`, orchestrator `v*`, quality gate incomplet | un singur release contract și same-commit matrix Windows/Linux/helper |
| `P2P-H01` | un singur seed DNS, fără PeerId pin-uit și fără discovery activ | seed/DNS compromise poate eclipsa noduri noi | `configs/mainnet-candidate.toml:31`, `DialOpts::unknown_peer_id()` | 3-5 seed-uri independente, PeerId pinning, Kademlia/PEX și failover |
| `P2P-H02` | inbound poate ocupa 127/128 sloturi; limitele sunt per PeerId, nu IP/subnet | Sybil poate elimina aproape toate conexiunile outbound | `max_inbound=max_peers-1`, total `max_peers` | outbound reservation și admission per IP/subnet/ASN |
| `P2P-H03` | discovery, reputație și istoricul gossip nu au TTL/cap/eviction | churn Sybil produce creștere continuă RAM/disc | `discovered_peers`, `ReputationStore.peers`, `published_transactions` | LRU/TTL/cap și compactare persistentă |
| `P2P-H04` | motive remote de reject pot fi mari și sunt persistate integral | amplificare RAM/disc per PeerId | response codec 16 MiB și `last_reason` persistent | motive structurate, capped, sanitize și metrici agregate |
| `P2P-H05` | gossip-ul este propagat fără manual app validation | invalid gossip poate fi amplificat și consuma CPU | lipsește `validate_messages()` și report validation result | manual validation + Gossipsub scoring |
| `P2P-H06` | „header-first” nu verifică PoW complet și recompensează înaintea bodies | high-work Hello poate ocupa branch slots | `HeaderSummary` incomplet și `validate_header_chain` structural | header complet, PoW prefilter și reward după body valid; diff separat |
| `P2P-H07` | sync response nu este corelat cu request/faza | state machine poate accepta tipuri nesolicitate în faza curentă | request ID este ignorat în response event | map `RequestId -> ExpectedResponse`, timeout și o ramură/peer |
| `P2P-H08` | sync requests recitesc întregul blockchain fără rate limit | CPU/RAM/I/O DoS în swarm handler | `load_blocks()` în FindAncestor/Headers/Blocks | query-uri point/range, worker pool bounded și token buckets |
| `RPC-H01` | bind safety verifică doar șirul `0.0.0.0` | `[::]` sau IP public specific poate expune profil local/mining | `RpcConfig::validate` face string equality | parse `IpAddr`, impune `is_loopback()` pentru local și teste IPv4/IPv6 |
| `RPC-H02` | rate-limit-ul are încredere în XFF și map-ul nu are cap/TTL | spoofing bypass + memory growth | `client_key()` preferă primul XFF | trusted-proxy allowlist, peer socket implicit, bounded cache |
| `RPC-H03` | 29 read routes nu au rate/concurrency/timeout/load-shed | response/CPU/RAM DoS | `/state`, `/mempool`, `/indexer/summary`, linear tx lookup | middleware global/per-route și endpoint-uri bounded/paginate |
| `RPC-H04` | deployment-ul folosește backend `private-mining` pentru RPC-ul public | mining public, dar `POST /transactions` este 404 | compose + verificare live | instanțe separate public-read/public-submit/private-mining și routing explicit |
| `RPC-H05` | RPC public este degradat operațional | timeout-uri, zero peers și index lag împiedică wallet/explorer/mining stabil | live 2026-07-28: vezi secțiunea 10 | diagnosticați CPU/locks/indexer/P2P, apoi SLO și probe continue |

## 7. Finding-uri Medium

| ID | DESCRIERE | IMPACT | REPRODUCERE / DOVADĂ | REMEDIERE |
|---|---|---|---|---|
| `SUP-M01` | `RUSTSEC-2026-0118` în Hickory NSEC3 | posibil OOM/loop dacă DNSSEC devine activ | audit + feature tree; DNSSEC nu este activ acum | păstrați DNSSEC dezactivat până la Hickory fix și adăugați feature gate test |
| `SUP-M02` | `rustls-pemfile 2.2.0` direct este unmaintained | mentenanță/security fixes absente în pool | cargo audit warning | migrare la parserul recomandat de rustls ecosystem |
| `QLT-M01` | proiectul nu interzice unsafe și Geiger are zone mari FFI | regresii unsafe nu sunt blocate automat | core 62 expr, miner 154 expr, helper 113 expr | `forbid(unsafe_code)` unde posibil și module FFI izolate/documentate |
| `QLT-M02` | runtime panic paths în miner/node | mutex poison sau invariant DB poate opri servicii | 7 Stratum expects, CUDA joins, 2 storage invariants | erori controlate, poison recovery și tests de fault injection |
| `REL-M01` | updater Desktop nu este activ | patch-urile de securitate nu au canal automat | Tauri config fără updater/plugin | updater semnat cu rollback și canal stable/candidate |
| `REL-M02` | output-urile Linux folosesc căi neuniforme | risc de artefact stale sau lipsă | `release-artifacts/linux-v2` vs `alvenqis-release/apps/linux` | un singur staging dir curățat și manifest SHA256 |
| `REL-M03` | uninstall Linux nu invocă purge complet | date reziduale și reinstalare inconsistentă | hook-urile nu rulează `--purge-uninstall` | uninstall idempotent care șterge app data doar cu confirmare explicită |
| `P2P-M01` | reputația nu controlează peer selection/mesh/sync și Ping nu alimentează scorul | peerii slabi rămân echivalenți cu peerii buni | event handling și branch selection | scoring compus latency/validity/uptime și selecție diversă |
| `P2P-M02` | penalizările inbound/outbound sunt asimetrice | abuz inbound poate evita reputația | handler request vs response | protocol fault taxonomy și penalizare consistentă |
| `P2P-M03` | `/p2p/status` poate expune topologia completă | facilitează reconnaissance | schema include address, PeerId, score, uptime | endpoint public agregat, detalii doar admin |
| `P2P-M04` | limitele VPS și node diferă; readiness implicit permite zero peers | deployment poate porni invalid sau declara ready fără rețea | VPS 256 vs node 128, min peers 0 | o singură schemă config și readiness mainnet >0 |
| `RPC-M01` | producția nu configurează API token | endpoint-uri costisitoare depind exclusiv de rate limits | template/compose fără token | credential intern între edge și private mining backend |
| `RPC-M02` | CORS nu permite headerele de auth implementate | browser clients nu pot folosi Bearer/X token | preflight live permite doar Content-Type | include headerele doar pentru origins explicite și teste |
| `RPC-M03` | P2P status public conține schema de topologie | reconnaissance și privacy leak | route publică în toate profilele | DTO agregat/redactat |
| `RPC-M04` | config guard verifică `self.api_token`, nu env effective token | config valid cu env poate fi refuzat sau tratat inconsistent | `RpcConfig::validate` vs `effective_api_token` | rezolvați secretul o dată la startup, apoi validați valoarea efectivă |
| `RPC-M05` | rate limits pot fi 0 pe profile publice | operatorul poate dezactiva protecția fără fail-fast | config acceptă 0 | refuz/warning explicit și override „trusted edge” separat |
| `RPC-M06` | health declară că mining nu are port public, deși edge îl publică | stare operațională falsă | `/health` live vs gateway routes | raportați exposure efectiv derivat din deployment |
| `RPC-M07` | edge-ul Alvenqis este Nginx și trust CF header nu este ancorat | neconformitate reform + trust boundary fragil | `docker/gateway/nginx.conf.template` | înlocuire prin Pingora la Faza 4; nu modificați 1Panel |

## 8. Finding-uri Low

| ID | DESCRIERE | IMPACT | REPRODUCERE / DOVADĂ | REMEDIERE |
|---|---|---|---|---|
| `SUP-L01` | `paste 1.0.15` este unmaintained, tranzitiv | debt supply-chain | cargo audit warning | eliminați prin upgrade libp2p/netlink când devine disponibil |
| `P2P-L01` | identitatea Windows nu are ACL explicit/atomic create-new | hardening local incomplet | branch Windows din identity persistence | ACL user-only și create-new/atomic replace |
| `P2P-L02` | hashrate și roluri sunt auto-declarate | telemetrie poate fi falsificată | mining presence payload | marcați explicit „unverified” și nu folosiți pentru decizii |
| `RPC-L01` | token RPC este comparat cu `==` | timing defense-in-depth incomplet | `require_write_auth` | `subtle::ConstantTimeEq` |
| `RPC-L02` | README indică două documente inexistente | integrare/operator guidance incomplet | `docs/api/*` lipsește | generați OpenAPI și docs versionate |
| `RPC-L03` | 429 nu are Retry-After și buckets sunt process-local | client retry și scale-out inconsistente | middleware rate limit | Retry-After, metrici și limiter coordonat/edge |

## 9. Acoperire P2P și RPC

### 9.1 P2P

Teste trecute:

```text
11 P2P unit/integration tests
10 peer reputation tests
2 devnet handshake tests
```

Lipsesc:

- Sybil/eclipsă multi-PeerId;
- outbound reservation;
- IP/subnet/ASN diversity;
- pinned seed failover;
- request flood;
- response correlation;
- invalid-gossip propagation;
- bounded reputation/discovery;
- DNS hijack.

### 9.2 RPC

Teste trecute:

```text
21 RPC integration tests
3 config --check-config profiles
```

Lipsesc:

- auth 401/accept valid token;
- 429 și window reset;
- XFF spoofing;
- IPv6/public bind guards;
- timeout/concurrency/load shedding;
- response size bounds;
- separare multi-instance a profilelor.

## 10. Evidență live RPC

Verificări efectuate la 2026-07-28:

```text
chain initialized: true
height: 490
blocks: 491
index height: 275
index lag: 215
connected peers: 0
validated peers: 0
```

Timpi observați:

| ENDPOINT | OBSERVAȚIE |
|---|---|
| `/health` | 17,635 s într-o serie; timeout >15 s în următoarea |
| `/network` | 0,547 s |
| `/status` | 0,728-1,322 s |
| `/sync/status` | timeout >20 s și >15 s |
| `/chain/tip` | 8,379 s |
| `/p2p/status` | 0,478 s, apoi timeout >15 s |
| `POST /transactions` cu `{}` | 404; ruta nu este înregistrată |
| `/mining/template` fără query | 400; ruta este publicată |

Aceste rezultate sunt snapshot-uri și pot varia. Ele sunt suficiente pentru a respinge orice declarație curentă de „RPC stable” sau „zero downtime”.

## 11. Prioritate de remediere

### P0 - înainte de orice redeploy public

1. separați profilele RPC public-submit și private-mining;
2. eliminați bind bypass și trust-ul necondiționat în forwarding headers;
3. adăugați read rate/concurrency/timeout/load shedding;
4. restabiliți P2P cu minimum peers >0 și investigați index lag;
5. nu publicați un release cu audit/deny/Clippy gates roșii.

### P1 - fundație și P2P

1. point/range storage queries și worker pools bounded;
2. pinned multi-seed bootstrap și discovery activ;
3. outbound reservation și diversitate IP/subnet/ASN;
4. manual gossip validation și sync response state machine;
5. TTL/LRU pentru toate structurile per peer/client.

### P2 - supply-chain, Desktop și release

1. remediați Hickory/glib/rustls-pemfile;
2. eliminați panic paths runtime prioritare;
3. activați updater semnat;
4. unificați tagurile, staging-ul și release matrix;
5. implementați uninstall/purge complet și idempotent.

## 12. Mapare la reform.md

| FINDING GROUP | TASK ȚINTĂ |
|---|---|
| storage/full-chain reads | 2.1-2.4 |
| backup/integritate | 2.5-2.6 |
| P2P/Sybil/eclipse/scoring | 3.1-3.6 |
| Nginx -> Pingora | 4.1-4.7 |
| dashboards/control/security | 5.1-5.6 |
| miner/Stratum | 6.1-6.6 |
| RPC security/versioning | 7.1-7.5 |
| packaging/release/docs | 8.1-8.6 |
| audit final/fuzz/pentest | 9.1-9.6 |

## 13. Condiții minime pentru închiderea finding-urilor

Un finding se închide numai dacă există:

1. diff tehnic identificabil;
2. test specific care eșuează înainte și trece după;
3. build/test exit code 0;
4. pentru deployment, probe live de health/latency/P2P/indexer/logs;
5. actualizare în documentația centrală;
6. fără regressions în serviciile protejate.

Modificările consensus-sensitive necesită diff separat și aprobare explicită înainte de implementare.

## 14. Concluzie

Faza 1 este completă ca analiză și raportare. Nu este completă ca remediere.

Raportul nu autorizează:

- eticheta `VERIFIED - 0 Vulnerabilities`;
- `MAINNET READY`;
- ștergerea sau redeploy-ul VPS înaintea task-urilor operaționale din reform.md;
- modificări de consens fără aprobare;
- atingerea `1Panel` sau a serviciilor protejate.

Următorul pas strict este Faza 2, Task 2.1, după actualizarea trackerului `reform.md`.
