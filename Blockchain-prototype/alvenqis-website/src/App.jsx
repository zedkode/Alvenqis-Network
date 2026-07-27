import { lazy, Suspense, useEffect } from 'react'
import Layout, { useRoutePath } from './components/layout/Layout.jsx'

const AdminApp = lazy(() => import('./admin/AdminApp.jsx'))
const CorePage = lazy(() => import('./pages/CorePage.jsx'))
const DevelopersPage = lazy(() => import('./pages/DevelopersPage.jsx'))
const DesktopPage = lazy(() => import('./pages/DesktopPage.jsx'))
const DocsPage = lazy(() => import('./pages/DocsPage.jsx'))
const EcosystemPage = lazy(() => import('./pages/EcosystemPage.jsx'))
const ExplorerPage = lazy(() => import('./pages/ExplorerPage.jsx'))
const FaqPage = lazy(() => import('./pages/FaqPage.jsx'))
const HomePage = lazy(() => import('./pages/HomePage.jsx'))
const NetworkPage = lazy(() => import('./pages/NetworkPage.jsx'))
const PassportPage = lazy(() => import('./pages/PassportPage.jsx'))
const ProtocolPage = lazy(() => import('./pages/ProtocolPage.jsx'))
const RoadmapPage = lazy(() => import('./pages/RoadmapPage.jsx'))
const StatusPage = lazy(() => import('./pages/StatusPage.jsx'))
const TokenomicsPage = lazy(() => import('./pages/TokenomicsPage.jsx'))
const WhitepaperPage = lazy(() => import('./pages/WhitepaperPage.jsx'))

const pages = {
  '/': <HomePage />,
  '/core': <CorePage />,
  '/explorer': <ExplorerPage />,
  '/developers': <DevelopersPage />,
  '/desktop': <DesktopPage />,
  '/tokenomics': <TokenomicsPage />,
  '/faq': <FaqPage />,
  '/network': <NetworkPage />,
  '/protocol': <ProtocolPage />,
  '/passport': <PassportPage />,
  '/ecosystem': <EcosystemPage />,
  '/whitepaper': <WhitepaperPage />,
  '/roadmap': <RoadmapPage />,
  '/status': <StatusPage />,
}

const productRedirects = {
  '/wallet': '/desktop#wallet',
  '/mining': '/desktop#mining',
  '/downloads': '/desktop#downloads',
}

function ProductRedirect({ target }) {
  useEffect(() => {
    window.history.replaceState({}, '', target)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }, [target])

  return <div className="grid min-h-[60vh] place-items-center text-frost">Opening Alvenqis Desktop...</div>
}

export default function App() {
  const path = useRoutePath()
  if (path === '/admin' || path.startsWith('/admin/')) {
    return (
      <Suspense fallback={<div className="grid min-h-screen place-items-center bg-void text-frost">Loading admin...</div>}>
        <AdminApp path={path} />
      </Suspense>
    )
  }

  const page = productRedirects[path]
    ? <ProductRedirect target={productRedirects[path]} />
    : path === '/docs' || path.startsWith('/docs/')
    ? <DocsPage path={path} />
    : pages[path] || <HomePage />

  return (
    <Layout path={path}>
      <Suspense fallback={<div className="grid min-h-[60vh] place-items-center text-frost">Loading page...</div>}>
        {page}
      </Suspense>
    </Layout>
  )
}
