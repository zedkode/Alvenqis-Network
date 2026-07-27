import { useCallback, useEffect, useMemo, useState } from 'react'
import * as staticContent from '../data/content.js'

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:4000'
const contentCache = new Map()

const pageSections = {
  global: ['navItems', 'networkStats', 'standards', 'openDecisions'],
  home: ['confirmationModel', 'ecosystemProducts', 'offChainItems', 'onChainItems', 'productLayers', 'roadmap', 'standards'],
  core: ['chainFacts', 'confirmationModel', 'coreModules', 'openDecisions'],
  mining: ['miningModules', 'networkStats'],
  wallet: ['walletFeatures'],
  explorer: ['explorerFeatures'],
  developers: ['developerStack', 'standards'],
  network: ['productLayers'],
  protocol: ['networkStats', 'openDecisions'],
  passport: ['passportUseCases'],
  ecosystem: ['ecosystemProducts', 'standards'],
  whitepaper: ['whitepaperSections'],
  roadmap: ['roadmap'],
  docs: ['docsCards'],
  status: ['openDecisions'],
  tokenomics: ['tokenomicsRows'],
  faq: ['faqItems'],
}

function staticSectionsFor(pageSlug) {
  const keys = [...(pageSections.global || []), ...(pageSections[pageSlug] || [])]

  return keys.reduce((sections, key) => {
    if (staticContent[key] !== undefined) {
      sections[key] = staticContent[key]
    }

    return sections
  }, {})
}

async function fetchPageContent(pageSlug, lang) {
  const response = await fetch(`${API_BASE_URL}/api/content/${pageSlug}?lang=${encodeURIComponent(lang)}`)

  if (!response.ok) {
    throw new Error(`CMS request failed with ${response.status}`)
  }

  const payload = await response.json()
  return payload.sections || {}
}

export function useContent(pageSlug, { lang = 'en' } = {}) {
  const fallbackSections = useMemo(() => staticSectionsFor(pageSlug), [pageSlug])
  const cacheKey = `${lang}:${pageSlug}`
  const [state, setState] = useState(() => {
    const cached = contentCache.get(cacheKey)

    return {
      sections: cached || fallbackSections,
      isLoading: !cached,
      error: null,
      source: cached ? 'cms-cache' : 'static-fallback',
    }
  })

  const load = useCallback(async () => {
    setState((current) => ({ ...current, isLoading: true, error: null }))

    try {
      const cmsSections = await fetchPageContent(pageSlug, lang)
      const sections = { ...fallbackSections, ...cmsSections }
      contentCache.set(cacheKey, sections)
      setState({ sections, isLoading: false, error: null, source: 'cms' })
    } catch (error) {
      setState({
        sections: fallbackSections,
        isLoading: false,
        error,
        source: 'static-fallback',
      })
    }
  }, [cacheKey, fallbackSections, lang, pageSlug])

  useEffect(() => {
    const cached = contentCache.get(cacheKey)

    if (cached) {
      setState({ sections: cached, isLoading: false, error: null, source: 'cms-cache' })
      return
    }

    load()
  }, [cacheKey, load])

  return {
    content: state.sections,
    sections: state.sections,
    isLoading: state.isLoading,
    error: state.error,
    source: state.source,
    refetch: load,
  }
}
