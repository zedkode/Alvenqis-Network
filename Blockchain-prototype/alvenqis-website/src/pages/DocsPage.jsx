import { useMemo, useState } from 'react'
import { ArrowLeft, BookOpen, Search } from 'lucide-react'
import { PageHero } from '../components/ui/PageShell.jsx'
import MarkdownDocument from '../components/docs/MarkdownDocument.jsx'
import {
  documentPathFromUrl,
  documents,
  documentsByPath,
  hrefForDocument,
} from '../docs/catalog.js'

function DocumentReader({ document }) {
  return (
    <>
      <PageHero
        eyebrow={`${document.section} / ${document.classification}`}
        title={document.title}
        text={`${document.status} · Source: ${document.path}`}
      >
        <a className="inline-flex items-center gap-2 text-sm font-bold text-ionHot" href="/docs">
          <ArrowLeft size={16} /> Back to documentation
        </a>
      </PageHero>
      <section className="px-5 pb-24">
        <div className="glass-panel mx-auto max-w-5xl rounded-lg p-6 sm:p-10">
          <MarkdownDocument document={document} />
        </div>
      </section>
    </>
  )
}

export default function DocsPage({ path = '/docs' }) {
  const documentPath = documentPathFromUrl(path)
  const activeDocument = documentPath ? documentsByPath.get(documentPath) : null
  const [query, setQuery] = useState('')
  const [section, setSection] = useState('All')

  const sections = useMemo(
    () => ['All', ...new Set(documents.map((document) => document.section))],
    [],
  )

  const filteredDocuments = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase()
    return documents.filter((document) => {
      const matchesSection = section === 'All' || document.section === section
      const haystack = `${document.title} ${document.path} ${document.status} ${document.content}`.toLowerCase()
      return matchesSection && (!normalizedQuery || haystack.includes(normalizedQuery))
    })
  }, [query, section])

  if (activeDocument) return <DocumentReader document={activeDocument} />

  return (
    <>
      <PageHero
        eyebrow="Documentation"
        title="One searchable source for Alvenqis documentation."
        text="Read the repository Markdown directly in the web interface. Current implementation, accepted decisions, planned work, and historical records are labeled separately."
      />
      <section className="px-5 pb-24">
        <div className="mx-auto max-w-7xl">
          {documentPath && (
            <div className="mb-6 rounded-lg border border-amber-400/30 bg-amber-400/10 p-4 text-sm text-amber-100">
              This document is internal, historical-only, excluded from the public catalog, or does not exist.
            </div>
          )}
          <div className="glass-panel rounded-lg p-5 sm:p-7">
            <div className="grid gap-4 lg:grid-cols-[1fr_auto]">
              <label className="flex items-center gap-3 rounded-lg border border-line bg-black/20 px-4 py-3">
                <Search size={18} className="text-ionHot" />
                <span className="sr-only">Search documentation</span>
                <input
                  className="w-full bg-transparent text-white outline-none placeholder:text-frost/40"
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Search titles, paths, status, and content"
                  type="search"
                  value={query}
                />
              </label>
              <label className="flex items-center gap-3 text-sm text-frost/65">
                Section
                <select
                  className="rounded-lg border border-line bg-void px-4 py-3 text-white"
                  onChange={(event) => setSection(event.target.value)}
                  value={section}
                >
                  {sections.map((item) => <option key={item}>{item}</option>)}
                </select>
              </label>
            </div>
            <p className="mt-4 text-sm text-frost/50">
              {filteredDocuments.length} of {documents.length} public documents
            </p>
          </div>

          <div className="mt-8 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            {filteredDocuments.map((document) => (
              <a
                className="glass-panel rounded-lg p-6 transition hover:-translate-y-1 hover:border-ionSoft/40"
                href={hrefForDocument(document.path)}
                key={document.path}
              >
                <div className="flex items-start justify-between gap-4">
                  <span className="rounded-full border border-ionSoft/20 px-3 py-1 text-xs uppercase tracking-[0.16em] text-ionSoft/70">
                    {document.section}
                  </span>
                  <BookOpen className="text-ionHot" size={20} />
                </div>
                <h2 className="mt-6 text-xl font-black text-white">{document.title}</h2>
                <p className="mt-3 line-clamp-2 text-sm leading-6 text-frost/58">{document.status}</p>
                <p className="mt-4 break-all font-mono text-xs text-frost/36">{document.path}</p>
              </a>
            ))}
          </div>
        </div>
      </section>
    </>
  )
}
