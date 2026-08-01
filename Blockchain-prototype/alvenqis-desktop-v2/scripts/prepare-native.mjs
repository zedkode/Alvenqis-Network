import { spawnSync } from "node:child_process";
import process from "node:process";

const withSidecars = process.argv.slice(2).includes("--with-sidecars");
const isWindows = process.platform === "win32";
const command = isWindows ? "powershell.exe" : "bash";
const args = isWindows
  ? [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      "./scripts/prepare-native.ps1",
      ...(withSidecars ? ["-WithSidecars"] : []),
    ]
  : ["./scripts/prepare-native.sh", ...(withSidecars ? ["--with-sidecars"] : [])];

const result = spawnSync(command, args, {
  cwd: new URL("..", import.meta.url),
  stdio: "inherit",
});

if (result.error) {
  console.error(`Unable to start ${command}: ${result.error.message}`);
  process.exit(1);
}

if (result.signal) {
  console.error(`Native preparation stopped by signal ${result.signal}.`);
  process.exit(1);
}

process.exit(result.status ?? 1);
