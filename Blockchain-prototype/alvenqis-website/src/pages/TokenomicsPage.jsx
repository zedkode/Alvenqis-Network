import { Coins, TimerReset, TrendingUp } from 'lucide-react'
import { motion } from 'framer-motion'
import { PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import MainnetCandidateBadge from '../components/ui/MainnetCandidateBadge.jsx'
import { useContent } from '../hooks/useContent.js'
import { useNetworkStats } from '../hooks/useNetwork.js'

export default function TokenomicsPage() {
  const { content } = useContent('tokenomics')
  const { stats, source } = useNetworkStats()
  const tokenomicsRows = content.tokenomicsRows || []
  const supplyPercent = Math.min((Number(stats.currentSupply) / Number(stats.maxSupply)) * 100, 100) || 0

  return (
    <>
      <PageHero
        eyebrow="Tokenomics"
        title="A mineable ALVE economy with explicit open allocation decisions."
        text="The supply and reward math are defined, while genesis allocation, treasury and fee destination must remain visibly open until decided."
      >
        <MainnetCandidateBadge source={source} />
      </PageHero>
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-8 lg:grid-cols-[0.85fr_1.15fr] lg:items-center">
          <div className="glass-panel rounded-lg p-8">
            <Coins className="text-ionHot" />
            <h2 className="mt-6 text-4xl font-black text-white">60M ALVE capped supply.</h2>
            <p className="mt-5 leading-8 text-frost/65">
              Mainnet Candidate emission is currently {stats.currentSupply} ALVE out of {stats.maxSupply} ALVE. Values come from the Alvenqis RPC when the local candidate node is available.
            </p>
            <div className="mt-8 h-5 overflow-hidden rounded-full border border-line bg-void/60">
              <motion.div
                className="h-full bg-gradient-to-r from-ionHot to-violetCore"
                initial={{ scaleX: 0 }}
                whileInView={{ scaleX: 1 }}
                viewport={{ once: true }}
                transition={{ duration: 1.1, ease: 'easeOut' }}
                style={{ transformOrigin: 'left' }}
                animate={{ width: `${Math.max(supplyPercent, 0.5)}%` }}
              />
            </div>
            <div className="mt-5 grid gap-3 text-sm text-frost/64 sm:grid-cols-3">
              <span>Height: {stats.height >= 0 ? `#${stats.height}` : 'offline'}</span>
              <span>Reward: {stats.currentReward} ALVE</span>
              <span>Halving: {stats.halvingCountdown} blocks</span>
            </div>
          </div>
          <div className="grid gap-3">
            {tokenomicsRows.map(([label, value, text], index) => (
              <motion.div
                key={label}
                initial={{ opacity: 0, x: 22 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.06 }}
                className="grid gap-4 rounded-lg border border-line bg-white/[0.035] p-5 md:grid-cols-[160px_190px_1fr] md:items-center"
              >
                <span className="font-black text-ionHot">{label}</span>
                <span className="font-semibold text-white">{value}</span>
                <span className="text-sm leading-6 text-frost/60">{text}</span>
              </motion.div>
            ))}
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-4 md:grid-cols-3">
          {[
            [TimerReset, `${stats.blockTimeSeconds} sec blocks`, 'Block time is read from backend network_params.'],
            [TrendingUp, '~3 year halving', `${stats.halvingInterval} blocks/cycle with the candidate-chain countdown visible above.`],
            [Coins, 'Fee model open', 'Miner incentives after late halvings need explicit design.'],
          ].map(([Icon, title, text]) => (
            <div key={title} className="glass-panel rounded-lg p-6">
              <Icon className="text-ionHot" />
              <h3 className="mt-6 text-2xl font-black text-white">{title}</h3>
              <p className="mt-3 leading-7 text-frost/62">{text}</p>
            </div>
          ))}
        </div>
      </section>
    </>
  )
}
