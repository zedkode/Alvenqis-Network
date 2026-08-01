import { promises as fs } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url))
const repositoryRoot = path.resolve(scriptDirectory, '..', '..')
const supportedExtensions = new Set([
  '.adoc',
  '.cjs',
  '.css',
  '.html',
  '.js',
  '.jsx',
  '.json',
  '.md',
  '.mdx',
  '.mjs',
  '.ps1',
  '.py',
  '.rs',
  '.rst',
  '.sh',
  '.toml',
  '.ts',
  '.tsx',
  '.txt',
  '.yaml',
  '.yml',
])
const ignoredDirectories = new Set([
  '.alvenqis-local',
  '.agent',
  '.agents',
  '.aider',
  '.cache',
  '.claude',
  '.codex',
  '.cursor',
  '.git',
  '.gemini',
  '.gradle',
  '.idea',
  '.superdesign',
  '.superdev',
  '.venv',
  '.vs',
  '.vscode',
  '.windsurf',
  'build',
  'coverage',
  'dist',
  'node_modules',
  'release-artifacts',
  'release-staging',
  'target',
  'target-miner-test',
  'venv',
])
const excludedFiles = new Set([
  'Blockchain-scripts/docs/check-english-content.mjs',
])
// Narrow compatibility literals retained by internal governance documents:
// one historical source filename and the owner's documented Romanian aliases
// for the otherwise-English continuation command. They are not public prose.
const permittedNonEnglishLiterals = [
  'PLAN_IMBUNATATIRI_ALVENQIS_NETWORK.md',
  'PLAN_IMBUNATATIRI...md',
  'continuă dezvoltarea',
  'continuă',
]
const romanianFilenamePattern =
  /(?:^|[-_.])(CITESTE|CUM[-_]FACI|DECIZII|DOCUMENTATIE|FAZA|IMBUNATATIRI|INCEPUT|RETEA|ROMANA|ROMANESC)(?:[-_.]|$)/i
const romanianDiacriticsPattern = /[ăâîșțĂÂÎȘȚ]/
const distinctiveRomanianWordsPattern =
  /(?<![A-Za-z])(acest|aceasta|acestea|pentru|fara|trebuie|retea|retele|fisier|fisiere|sterge|instalare|dezinstalare|imbunatatiri|citeste|decizii|dovada|inceput|fiecare|inainte|nicio|niciun|ramane|continua|asteptarea|verificari|oricare)(?![A-Za-z])/iu

function repositoryPath(absolutePath) {
  return path.relative(repositoryRoot, absolutePath).split(path.sep).join('/')
}

async function collectFiles(directory, files) {
  for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue
    const absolutePath = path.join(directory, entry.name)
    if (entry.isDirectory()) {
      await collectFiles(absolutePath, files)
    } else if (entry.isFile() && supportedExtensions.has(path.extname(entry.name).toLowerCase())) {
      files.push(absolutePath)
    }
  }
}

const files = []
await collectFiles(repositoryRoot, files)
const issues = []

for (const absolutePath of files.sort()) {
  const relativePath = repositoryPath(absolutePath)
  if (excludedFiles.has(relativePath)) continue
  if (romanianFilenamePattern.test(relativePath)) {
    issues.push(`${relativePath}: Romanian filename`)
  }

  const content = await fs.readFile(absolutePath, 'utf8')
  for (const [index, line] of content.split(/\r?\n/).entries()) {
    const checkedLine = permittedNonEnglishLiterals.reduce(
      (value, literal) => value.replaceAll(literal, ''),
      line,
    )
    if (
      romanianDiacriticsPattern.test(checkedLine) ||
      distinctiveRomanianWordsPattern.test(checkedLine)
    ) {
      issues.push(`${relativePath}:${index + 1}: Romanian content`)
    }
  }
}

if (issues.length > 0) {
  console.error(`English-only source check failed with ${issues.length} issue(s):`)
  for (const issue of issues) console.error(`- ${issue}`)
  process.exit(1)
}

console.log(`English-only source check passed for ${files.length} source text files.`)
