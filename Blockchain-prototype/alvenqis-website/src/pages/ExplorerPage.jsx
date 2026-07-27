import { Activity, Blocks, Box, FileSearch, Radar, Route } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import MainnetCandidateBadge from '../components/ui/MainnetCandidateBadge.jsx'
import VisualPanel from '../components/ui/VisualPanel.jsx'
import { useContent } from '../hooks/useContent.js'
import { useNetworkBlocks, useNetworkStats } from '../hooks/useNetwork.js'

function shortHash(hash) {
  if (!hash) return 'pending'
  return `${hash.slice(0, 12)}...${hash.slice(-8)}`
}

export default function ExplorerPage() {
  const { content } = useContent('explorer')
  const { stats, source: statsSource } = useNetworkStats()
  const { blocks, source: blocksSource } = useNetworkBlocks({ limit: 6 })
  const explorerFeatures = content.explorerFeatures || []
  const icons = [Blocks, Route, Radar, Box, FileSearch, Activity]
  return (
    <>
      <PageHero
        eyebrow="Explorer"
        title="Explorer is the truth surface for blocks, transactions, assets and status."
        text="This public surface reads Mainnet Candidate blocks from the Alvenqis RPC while preserving honest launch status."
      >
        <MainnetCandidateBadge source={statsSource === 'rpc' && blocksSource === 'rpc' ? 'rpc' : 'fallback'} />
      </PageHero>
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-8 lg:grid-cols-[1fr_1fr] lg:items-center">
          <VisualPanel variant="explorer" kicker="Explorer visual" title="Blocks, events, assets and network health." />
          <div className="glass-panel rounded-lg p-8">
            <p className="font-mono text-xs uppercase tracking-[0.26em] text-ionSoft/80">Mainnet Candidate stats</p>
            <div className="mt-5 grid gap-3 sm:grid-cols-2">
              {[
                ['Height', stats.height >= 0 ? stats.height : 'offline'],
                ['Reward', `${stats.currentReward} ALVE`],
                ['Block time', `${stats.blockTimeSeconds}s`],
                ['Halving in', `${stats.halvingCountdown} blocks`],
              ].map(([label, value]) => (
                <div key={label} className="rounded-lg border border-line bg-void/60 p-4">
                  <div className="text-xs uppercase tracking-[0.22em] text-ionSoft/60">{label}</div>
                  <div className="mt-2 font-mono text-lg font-black text-white">{value}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Candidate chain" title="Latest Mainnet Candidate blocks." text="These rows come directly from the Alvenqis Rust RPC. They are candidate-chain data, not a public mainnet launch claim." />
          <div className="overflow-hidden rounded-lg border border-line">
            {(blocks.length ? blocks : [{ height: 'offline', hash: 'API unavailable', reward: '0.00000000', txCount: 0, timestamp: null }]).map((block) => (
              <div key={block.id || block.hash} className="grid gap-3 border-b border-line bg-white/[0.025] p-5 last:border-b-0 md:grid-cols-[120px_1fr_170px_100px] md:items-center">
                <span className="font-mono font-black text-ionHot">#{block.height}</span>
                <span className="font-mono text-sm text-frost/70">{shortHash(block.hash)}</span>
                <span className="font-mono text-sm text-white">{block.reward} ALVE</span>
                <span className="text-sm text-frost/58">{block.txCount} tx</span>
              </div>
            ))}
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Explorer features" title="What the explorer must eventually show." text="Explorer pages should become live only when API data, indexer sync and status checks exist." />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.08 }} className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {explorerFeatures.map(([title, text], index) => (
              <FeatureCard key={title} icon={icons[index]} eyebrow="Explorer" title={title} text={text} />
            ))}
          </motion.div>
        </div>
      </section>
    </>
  )
}
