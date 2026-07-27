import { promises as fs } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDirectory, '..', '..')
const publicDocsRoot = path.join(root, 'Blockchain-docs', 'human')
const manifest = JSON.parse(await fs.readFile(path.join(publicDocsRoot, 'documentation-manifest.json'), 'utf8'))
const inventoryPath = path.join(publicDocsRoot, 'DOCUMENTATION_INVENTORY.md')
const writeInventory = process.argv.includes('--write')

const ignoredDirectories = new Set([
  '.agents', '.artifacts', '.codex', '.git', '.review', '.alvenqis-local', '.tmp-firo',
  '.superdesign', '.vscode',
  'dist', 'node_modules', 'release-artifacts', 'target', 'alvenqis-docker',
])
const ignoredFiles = new Set([
  'AUDIT_ALVENQIS_NETWORK.md',
  'CODE_REVIEW_REPORT_2026-07-19.md',
  'PLAN_IMBUNATATIRI_ALVENQIS_NETWORK.md',
  'RUST_CODE_ANALYSIS_REPORT.md',
])
const ignoredPathPrefixes = [
  'Blockchain-docs/ai/',
]

async function walk(directory) {
  const results = []
  for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue
    if (entry.isFile() && ignoredFiles.has(entry.name)) continue
    const absolute = path.join(directory, entry.name)
    const relative = repoPath(absolute)
    if (ignoredPathPrefixes.some((prefix) => relative.startsWith(prefix))) continue
    if (entry.isDirectory()) results.push(...await walk(absolute))
    else if (/\.mdx?$/i.test(entry.name)) results.push(absolute)
  }
  return results
}

function repoPath(absolute) {
  return path.relative(root, absolute).split(path.sep).join('/')
}

function startsWithAny(value, prefixes) {
  return prefixes.some((prefix) => value.startsWith(prefix))
}

function isPublicSource(file) {
  if (file === 'README.md') return true
  if (file.startsWith('Blockchain-docs/human/')) return true
  if (file.startsWith('Blockchain-prototype/shared/')) return true
  if (/^Blockchain-prototype\/alvenqis-[^/]+\/README\.md$/.test(file)) return true
  return /^Blockchain-prototype\/alvenqis-sdk-rust\/docs\/.*\.md$/.test(file)
}

function classify(file) {
  if (startsWithAny(file, manifest.internal_prefixes)) return 'Internal'
  if (startsWithAny(file, manifest.historical_prefixes)) return 'Historical'
  if (isPublicSource(file)) return 'Public'
  return 'Component reference'
}

function firstMatch(content, expression, fallback = '') {
  return content.match(expression)?.[1]?.trim().replaceAll('|', '\\|') || fallback
}

function hasMojibake(content) {
  // Match common UTF-8-as-Windows-1252 sequences without rejecting legitimate
  // Romanian characters such as ă, â, î, ș and ț.
  return /\u00e2(?:\u20ac|\u2020)|\u00c3[\u0080-\u00bf]|\u00c2[\u0080-\u00bf]|\u0102\u02c6|\uFFFD/.test(content)
}

function localLinkTargets(content) {
  return [...content.matchAll(/!?(?:\[[^\]]*\])\(([^)\s]+)(?:\s+['"][^'"]*['"])?\)/g)]
    .map((match) => match[1].replace(/^<|>$/g, ''))
    .filter((target) => target && !/^(?:#|https?:|mailto:|data:)/i.test(target))
}

async function targetExists(sourceAbsolute, target) {
  const decoded = decodeURIComponent(target.split('#', 1)[0])
  if (!decoded) return true
  if (decoded === '/') return true
  const absolute = path.resolve(path.dirname(sourceAbsolute), decoded)
  try {
    const stat = await fs.stat(absolute)
    if (stat.isDirectory()) await fs.access(path.join(absolute, 'README.md'))
    return true
  } catch {
    return false
  }
}

const stalePatterns = [
  [/CPU \+ OpenCL GPU miner/i, 'removed CPU/OpenCL miner description'],
  [/standalone Blake3 miner/i, 'removed Blake3 PoW miner description'],
  [/recomputed Blake3 hashes/i, 'removed Blake3 pool validation description'],
  [/Tauri 0\.[0-9]+/i, 'stale Control Center version'],
  [/apply updates \*\*without operator confirmation\*\*/i, 'invalid desktop auto-install policy'],
  [/accepted launch PoW algorithm is Blake3/i, 'superseded PoW decision'],
  [/alvenqis-miner\/src\/backends\/opencl\.rs/i, 'removed OpenCL source path'],
  [/\bP2P (?:protocol )?v2\b/i, 'superseded P2P protocol version'],
  [/0000a26d0a9da9577f94350eaed9568f04e7e823f9e2ee5d0df0df52597779c2/i, 'superseded candidate genesis hash'],
  [/Rust implementation (?:starts|begins) only after/i, 'obsolete pre-implementation gate'],
  [/^threads\s*=\s*\d+/im, 'removed CPU-thread miner configuration'],
]

const files = (await walk(root)).sort((left, right) => repoPath(left).localeCompare(repoPath(right)))
const rows = []
const errors = []
const navigationFragments = new Set([
  'Blockchain-docs/human/_navbar.md',
  'Blockchain-docs/human/_sidebar.md',
])

for (const absolute of files) {
  const file = repoPath(absolute)
  const content = await fs.readFile(absolute, 'utf8')
  const classification = classify(file)
  const title = firstMatch(content, /^#\s+(.+)$/m, '(missing title)')
  const status = firstMatch(content, /^(?:\*\*)?Status(?:\*\*)?:\s*(.+)$/im, 'Not stated')

  rows.push({ file, classification, title, status })

  if (title === '(missing title)' && classification !== 'Internal' && !navigationFragments.has(file)) {
    errors.push(`${file}: missing level-one title`)
  }

  if (classification === 'Historical') {
    const opening = content.split('\n').slice(0, 20).join('\n')
    if (!/historical/i.test(opening) || !/not current/i.test(opening)) {
      errors.push(`${file}: historical document lacks a clear "historical / not current" opening banner`)
    }
  } else if (classification !== 'Internal') {
    if (hasMojibake(content)) {
      errors.push(`${file}: mojibake or invalid UTF-8 text`)
    }
    for (const [pattern, description] of stalePatterns) {
      if (pattern.test(content)) errors.push(`${file}: ${description}`)
    }
  }

  for (const target of localLinkTargets(content)) {
    if (!await targetExists(absolute, target)) errors.push(`${file}: broken local link ${target}`)
  }
}

const counts = rows.reduce((result, row) => {
  result[row.classification] = (result[row.classification] || 0) + 1
  return result
}, {})

const inventory = `# Alvenqis Documentation Inventory

Status: Generated audit inventory

Generated by \`node Blockchain-scripts/docs/audit-docs.mjs --write\`. Do not edit this
file by hand. Classification controls whether a document can appear in the
public Markdown reader; it does not change the historical content itself.

## Summary

| Class | Documents |
|---|---:|
${Object.entries(counts).sort().map(([name, count]) => `| ${name} | ${count} |`).join('\n')}
| **Total** | **${rows.length}** |

## Complete inventory

| Path | Class | Status | Title |
|---|---|---|---|
${rows.map((row) => `| \`${row.file}\` | ${row.classification} | ${row.status} | ${row.title} |`).join('\n')}
`

if (writeInventory) await fs.writeFile(inventoryPath, inventory, 'utf8')
else {
  try {
    const current = await fs.readFile(inventoryPath, 'utf8')
    if (current !== inventory) errors.push('Blockchain-docs/human/DOCUMENTATION_INVENTORY.md is stale; run the audit with --write')
  } catch {
    errors.push('Blockchain-docs/human/DOCUMENTATION_INVENTORY.md is missing; run the audit with --write')
  }
}

console.log(`Audited ${rows.length} Markdown documents.`)
console.log(Object.entries(counts).sort().map(([name, count]) => `${name}: ${count}`).join(', '))

if (errors.length) {
  console.error(`Documentation audit failed with ${errors.length} issue(s):`)
  for (const error of errors) console.error(`- ${error}`)
  process.exitCode = 1
} else {
  console.log('Documentation audit passed.')
}
