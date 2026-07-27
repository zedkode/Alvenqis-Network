# alvenqis-browser

Status: Prototype / Mainnet Candidate / not store-ready

The browser prototype combines a Manifest V3 extension UI with a Rust native
messaging host. The extension never stores mnemonics or private keys; encrypted
keystore, signing, account composition, and RPC submission stay in the host.

## Security boundary

- default keystore: `.alvenqis-mainnet/browser-host/wallets/` under the user home;
- Argon2id plus AES-256-GCM encrypted wallet file;
- **CR-C01**: `create_wallet` never returns the recovery phrase over native messaging.
  The host shows the phrase once via OS dialog (Windows MessageBox) or stderr +
  `ALVENQIS_HOST_RECOVERY_ACK=1` (headless). Prefer CLI `--init-wallet` for offline
  backup. `create_session` is disabled (it discarded recovery material).
- **CR-C02**: `prepare_and_sign` / `send` / `submit` require OS confirmation **by
  default**. Opt out only with `--no-require-os-confirm` or
  `ALVENQIS_HOST_REQUIRE_OS_CONFIRM=0` (dev/test only — unsafe for funds).
- mnemonic never appears in extension UI or JSON responses;
- no mining, pool worker, WASM private-key, or unauthenticated dApp-connect path.

## Build and inspect

```powershell
cargo build -p alvenqis-browser-host --release
cargo run -p alvenqis-browser-host -- --print-info
cargo run -q -p alvenqis-browser-host -- --check-health --json
```

Create a recoverable encrypted wallet from the host CLI:

```powershell
cargo run -p alvenqis-browser-host -- --init-wallet --passphrase "your-long-passphrase"
cargo run -p alvenqis-browser-host -- --export-public
```

Import recovery words only through the CLI:

```powershell
cargo run -p alvenqis-browser-host -- --import-mnemonic --mnemonic "word1 word2 ..." --passphrase "..."
```

## Register the native host on Windows

1. Load `alvenqis-browser/extension` as an unpacked Chrome/Edge extension.
2. Copy its 32-character extension ID.
3. Run:

```powershell
.\scripts\browser\register-native-host.ps1 -ExtensionId <id> -Build -Browser Chrome
```

Use `-Browser All` or `-LocalRpc` only when required. OS confirm for send/sign/submit
is **on by default** in the host; use `-NoOsConfirm` only for automated dev/test.
Remove the registration with:

```powershell
.\scripts\browser\unregister-native-host.ps1 -Browser All -RemoveInstallDir
```

Linux registration:

```bash
./scripts/browser/register-native-host.sh --extension-id <id> --build --browser chrome
```

## Development protocol

Run `cargo run -p alvenqis-browser-host -- --jsonl --local` and send line-delimited
JSON requests such as `{"id":1,"method":"ping"}`. Native browser mode uses the
standard little-endian `u32` length plus UTF-8 JSON framing.

See `../docs/architecture/07_BROWSER_EXTENSION_AND_NATIVE_HOST.md` for the
method and trust-boundary summary.
