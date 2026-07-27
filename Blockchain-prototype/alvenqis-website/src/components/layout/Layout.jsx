import { useEffect, useState } from 'react'
import { ArrowRight, Menu, X } from 'lucide-react'
import { useContent } from '../../hooks/useContent.js'
import useSmoothScroll from '../../hooks/useSmoothScroll.js'
import GlobalEffects from '../effects/GlobalEffects.jsx'
import ScrollProgress from '../effects/ScrollProgress.jsx'
import CursorGlow from '../effects/CursorGlow.jsx'
import Preloader from '../effects/Preloader.jsx'
import Logo from '../ui/Logo.jsx'

const internalRoutes = [
  '/',
  '/network',
  '/protocol',
  '/ecosystem',
  '/whitepaper',
  '/docs',
  '/tokenomics',
  '/faq',
  '/core',
  '/desktop',
  '/mining',
  '/wallet',
  '/explorer',
  '/passport',
  '/developers',
  '/downloads',
  '/roadmap',
  '/status',
  '/admin',
  '/admin/users',
  '/admin/content',
  '/admin/network',
  '/admin/roadmap',
  '/admin/faq',
  '/admin/audit',
]

function isActive(currentPath, href) {
  if (href === '/') return currentPath === '/'
  return currentPath.startsWith(href)
}

export function useRoutePath() {
  const [path, setPath] = useState(window.location.pathname)

  useEffect(() => {
    const onPopState = () => setPath(window.location.pathname)
    const onClick = (event) => {
      const link = event.target.closest('a')
      if (!link) return

      const url = new URL(link.href)
      const internalRoute = internalRoutes.includes(url.pathname) || url.pathname.startsWith('/docs/')
      if (url.origin !== window.location.origin || !internalRoute) return

      event.preventDefault()
      window.history.pushState({}, '', `${url.pathname}${url.hash}`)
      setPath(url.pathname)

      requestAnimationFrame(() => {
        if (url.hash) {
          document.querySelector(url.hash)?.scrollIntoView({ behavior: 'smooth' })
          return
        }
        window.scrollTo({ top: 0, behavior: 'smooth' })
      })
    }

    window.addEventListener('popstate', onPopState)
    document.addEventListener('click', onClick)
    return () => {
      window.removeEventListener('popstate', onPopState)
      document.removeEventListener('click', onClick)
    }
  }, [])

  return path
}

function Header({ path }) {
  const [open, setOpen] = useState(false)
  const { content } = useContent('global')
  const staleProductRoutes = new Set(['/wallet', '/mining', '/downloads'])
  const navItems = (content.navItems || []).filter((item) => !staleProductRoutes.has(item.href))

  useEffect(() => {
    setOpen(false)
  }, [path])

  return (
    <header className="fixed left-0 right-0 top-0 z-40 border-b border-line bg-void/72 backdrop-blur-xl">
      <nav className="mx-auto flex max-w-7xl items-center justify-between px-5 py-4">
        <a href="/" aria-label="Alvenqis Network home">
          <Logo />
        </a>

        <div className="hidden items-center gap-1 rounded-full border border-line bg-white/[0.035] p-1 xl:flex">
          {navItems.map((item) => (
            <a
              key={item.href}
              href={item.href}
              className={`rounded-full px-3 py-2 text-xs font-semibold transition ${
                isActive(path, item.href)
                  ? 'bg-ionSoft/12 text-ionHot'
                  : 'text-frost/66 hover:bg-white/[0.05] hover:text-white'
              }`}
            >
              {item.label}
            </a>
          ))}
        </div>

        <div className="flex items-center gap-3">
          <a
            href="/status"
            className="hidden items-center gap-2 rounded-full border border-ionSoft/30 px-4 py-2 text-sm font-bold text-ionHot transition hover:bg-ionSoft/10 sm:inline-flex"
          >
            Build status <ArrowRight size={16} />
          </a>
          <button
            type="button"
            aria-label="Toggle menu"
            onClick={() => setOpen((value) => !value)}
            className="grid h-11 w-11 place-items-center rounded-lg border border-line text-frost xl:hidden"
          >
            {open ? <X size={20} /> : <Menu size={20} />}
          </button>
        </div>
      </nav>

      {open && (
        <div className="border-t border-line bg-void/96 px-5 py-4 xl:hidden">
          <div className="mx-auto grid max-w-7xl gap-2">
            {navItems.map((item) => (
              <a
                key={item.href}
                href={item.href}
                className={`rounded-lg px-4 py-3 font-semibold ${
                  isActive(path, item.href)
                    ? 'bg-ionSoft/12 text-ionHot'
                    : 'text-frost/70 hover:bg-white/[0.04]'
                }`}
              >
                {item.label}
              </a>
            ))}
          </div>
        </div>
      )}
    </header>
  )
}

function Footer() {
  const footerGroups = [
    ['Network', [['Core', '/core'], ['Protocol', '/protocol'], ['Roadmap', '/roadmap']]],
    ['Product', [['Desktop', '/desktop'], ['Explorer', '/explorer'], ['Passport', '/passport'], ['Ecosystem', '/ecosystem']]],
    ['Docs', [['Developers', '/developers'], ['Whitepaper', '/whitepaper'], ['Tokenomics', '/tokenomics'], ['FAQ', '/faq'], ['Status', '/status']]],
  ]

  return (
    <footer className="border-t border-line px-5 py-12">
      <div className="mx-auto grid max-w-7xl gap-8 md:grid-cols-[1fr_1.2fr] md:items-start">
        <div>
          <Logo />
          <p className="mt-5 max-w-md text-sm leading-7 text-frost/58">
            The public entry point for Alvenqis Network, its desktop control center, explorer, protocol documentation and candidate-network status.
          </p>
        </div>
        <div className="grid gap-4 sm:grid-cols-3">
          {footerGroups.map(([title, items]) => (
            <div key={title}>
              <h3 className="text-sm font-black uppercase tracking-[0.22em] text-ionSoft/70">{title}</h3>
              <div className="mt-4 grid gap-2 text-sm text-frost/58">
                {items.map(([item, href]) => (
                  <a key={item} href={href} className="transition hover:text-ionHot">{item}</a>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
      <div className="mx-auto mt-10 flex max-w-7xl flex-col gap-3 border-t border-line pt-6 text-xs text-frost/42 sm:flex-row sm:items-center sm:justify-between">
        <span>Alvenqis Network / ALVE · Mainnet Candidate.</span>
        <span>Wallet and mining live inside Alvenqis Desktop. The web explorer remains read-only.</span>
      </div>
    </footer>
  )
}

export default function Layout({ children, path }) {
  useSmoothScroll()

  return (
    <div className="min-h-screen bg-void text-frost">
      <ScrollProgress />
      <Preloader />
      <CursorGlow />
      <div className="noise" />
      <GlobalEffects />
      <Header path={path} />
      <main className="relative z-10">{children}</main>
      <Footer />
    </div>
  )
}
