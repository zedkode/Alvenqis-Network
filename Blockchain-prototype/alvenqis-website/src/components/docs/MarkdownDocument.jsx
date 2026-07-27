import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { resolveDocumentLink } from '../../docs/catalog.js'

export default function MarkdownDocument({ document }) {
  return (
    <article className="docs-markdown" data-document-path={document.path}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a({ href, children, ...props }) {
            const resolvedHref = resolveDocumentLink(document.path, href)
            const external = /^https?:/i.test(resolvedHref || '')
            return (
              <a
                {...props}
                href={resolvedHref}
                rel={external ? 'noreferrer noopener' : undefined}
                target={external ? '_blank' : undefined}
              >
                {children}
              </a>
            )
          },
        }}
      >
        {document.content}
      </ReactMarkdown>
    </article>
  )
}
