import documentationManifest from '../../../../Blockchain-docs/human/documentation-manifest.json'

const markdownModules = import.meta.glob(
  [
    '../../../../README.md',
    '../../../../Blockchain-docs/human/**/*.md',
    '../../../shared/**/*.md',
    '../../../alvenqis-*/README.md',
    '../../../alvenqis-sdk-rust/docs/**/*.md',
  ],
  { eager: true, import: 'default', query: '?raw' },
)

function repositoryPath(modulePath) {
  if (modulePath.startsWith('../../../../')) return modulePath.slice('../../../../'.length).replaceAll('\\', '/')
  if (modulePath.startsWith('../../../')) return `Blockchain-prototype/${modulePath.slice('../../../'.length)}`.replaceAll('\\', '/')
  return modulePath.replaceAll('\\', '/')
}

function firstMatch(content, pattern) {
  return content.match(pattern)?.[1]?.trim() || ''
}

function titleFromPath(path) {
  return path
    .split('/')
    .at(-1)
    .replace(/\.mdx?$/i, '')
    .replace(/^\d+[_-]/, '')
    .replaceAll(/[_-]+/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase())
}

function sectionFromPath(path) {
  if (path === 'README.md') return 'Workspace'
  if (path.startsWith('Blockchain-docs/human/')) {
    const section = path.split('/')[2]
    return section.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
  }
  return 'Components'
}

function classificationFor(path, status) {
  if (documentationManifest.historical_prefixes.some((prefix) => path.startsWith(prefix))) return 'Historical'
  if (/implemented/i.test(status)) return 'Implemented'
  if (/planned|research/i.test(status)) return 'Planned'
  if (/prototype|candidate/i.test(status)) return 'Candidate'
  return 'Reference'
}

export function isPublicDocument(path) {
  return !documentationManifest.internal_prefixes.some((prefix) => path.startsWith(prefix))
    && !documentationManifest.historical_prefixes.some((prefix) => path.startsWith(prefix))
}

export const documents = Object.entries(markdownModules)
  .map(([modulePath, content]) => {
    const path = repositoryPath(modulePath)
    const title = firstMatch(content, /^#\s+(.+)$/m) || titleFromPath(path)
    const status = firstMatch(content, /^(?:\*\*)?Status(?:\*\*)?:\s*(.+)$/im) || 'Reference'
    return {
      path,
      title,
      status: status.replaceAll('**', ''),
      section: sectionFromPath(path),
      classification: classificationFor(path, status),
      content,
    }
  })
  .filter((document) => isPublicDocument(document.path))
  .sort((left, right) => left.path.localeCompare(right.path))

export const documentsByPath = new Map(documents.map((document) => [document.path, document]))

export function hrefForDocument(path) {
  return `/docs/${path.split('/').map(encodeURIComponent).join('/')}`
}

export function documentPathFromUrl(pathname) {
  if (!pathname.startsWith('/docs/')) return null
  try {
    return pathname.slice('/docs/'.length).split('/').map(decodeURIComponent).join('/')
  } catch {
    return null
  }
}

function normalizeSegments(path) {
  const segments = []
  for (const segment of path.split('/')) {
    if (!segment || segment === '.') continue
    if (segment === '..') segments.pop()
    else segments.push(segment)
  }
  return segments.join('/')
}

export function resolveDocumentLink(sourcePath, href) {
  if (!href || href.startsWith('#') || /^(?:https?:|mailto:)/i.test(href)) return href

  const [pathPart, fragment] = href.split('#', 2)
  if (!/\.mdx?$/i.test(pathPart)) return href

  const sourceDirectory = sourcePath.includes('/') ? sourcePath.slice(0, sourcePath.lastIndexOf('/')) : ''
  const resolvedPath = normalizeSegments(`${sourceDirectory}/${decodeURIComponent(pathPart)}`)
  if (!documentsByPath.has(resolvedPath)) return href

  return `${hrefForDocument(resolvedPath)}${fragment ? `#${encodeURIComponent(fragment)}` : ''}`
}
