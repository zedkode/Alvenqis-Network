import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const desktopRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (relativePath) => fs.readFileSync(path.join(desktopRoot, relativePath), "utf8");
const packageVersion = JSON.parse(read("package.json")).version;
const expectedIndex = process.argv.indexOf("--expected");
const explicitExpected = expectedIndex >= 0 ? process.argv[expectedIndex + 1] : undefined;
const mismatches = [];

function capture(relativePath, pattern, label) {
  const match = read(relativePath).match(pattern);
  if (!match) {
    mismatches.push(`${label}: version field not found in ${relativePath}`);
    return;
  }
  if (match[1] !== packageVersion) {
    mismatches.push(
      `${label}: ${match[1]} does not match package.json ${packageVersion}`,
    );
  }
}

if (!/^\d+\.\d+\.\d+$/.test(packageVersion)) {
  mismatches.push(`package.json: ${packageVersion} is not X.Y.Z`);
}
if (expectedIndex >= 0 && !explicitExpected) {
  mismatches.push("--expected requires a version argument");
} else if (explicitExpected && explicitExpected !== packageVersion) {
  mismatches.push(
    `expected version ${explicitExpected} does not match package.json ${packageVersion}`,
  );
}

const tauriVersion = JSON.parse(read("src-tauri/tauri.conf.json")).version;
if (tauriVersion !== packageVersion) {
  mismatches.push(
    `tauri.conf.json: ${tauriVersion} does not match package.json ${packageVersion}`,
  );
}

capture("src-tauri/Cargo.toml", /^version\s*=\s*"([^"]+)"/m, "Cargo.toml");
capture("shared/constants.ts", /APP_VERSION\s*=\s*"([^"]+)"/, "APP_VERSION");
capture("packaging/arch/PKGBUILD", /^pkgver=([^\s]+)$/m, "PKGBUILD");
capture(
  "README.md",
  /\*\*Package:\*\*\s+`alvenqis-desktop-v2`\s+\(`([^`]+)`\)/,
  "README",
);

if (mismatches.length > 0) {
  console.error("Desktop version metadata is inconsistent:");
  for (const mismatch of mismatches) console.error(`  - ${mismatch}`);
  process.exit(1);
}

console.log(`Desktop version metadata is consistent: ${packageVersion}`);
