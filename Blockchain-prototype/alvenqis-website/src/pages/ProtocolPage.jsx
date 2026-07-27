import { AlertTriangle, Coins, Gauge, Lock, Pickaxe, ScrollText } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, MetricCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import { useContent } from '../hooks/useContent.js'

export default function ProtocolPage() {
  const { content } = useContent('protocol')
  const networkStats = content.networkStats || []
  const openDecisions = content.openDecisions || []
  return (
    <>
      <PageHero
        eyebrow="Protocol brief"
        title="Implemented candidate consensus with explicit remaining freezes."
        text="FiroPoW, DAA, accounts, addresses, fees and emission are implemented. Serialization, storage, contracts, governance and G4 approval remain explicit work."
      >
        <div className="grid max-w-4xl grid-cols-2 gap-3 sm:grid-cols-4">
          {networkStats.map((item) => (
            <MetricCard key={item.label} {...item} />
          ))}
        </div>
      </PageHero>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Fixed direction"
            title="PoW-first, Rust-based, ALVE-native."
            text="The current source defines the public economic direction and keeps PoLW as an upgrade path or research area, not a first-launch promise."
          />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.12 }} className="grid gap-4 md:grid-cols-3">
            {[
              [Pickaxe, 'Mineable supply', 'Block rewards start at 19.02587519 ALVE per block with a roughly three-year halving cycle.'],
              [Gauge, '60 second blocks', 'Designed for faster UX than classic 10-minute chains while keeping validation and operation manageable.'],
              [Coins, '60M max supply', 'The current economic model targets a capped supply of 60,000,000 ALVE.'],
            ].map(([icon, title, text]) => (
              <FeatureCard key={title} icon={icon} title={title} eyebrow="Defined" text={text} />
            ))}
          </motion.div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Open decisions"
            title="The remaining protocol and production gates."
            text="These points remain unresolved or incomplete and must not be inferred from the implemented candidate baseline."
          />
          <div className="grid gap-3 md:grid-cols-2">
            {openDecisions.map(([title, text]) => (
              <div key={title} className="rounded-lg border border-line bg-ink/70 p-5">
                <div className="flex items-start gap-4">
                  <AlertTriangle className="mt-1 shrink-0 text-ionHot" size={20} />
                  <div>
                    <h3 className="font-black text-white">{title}</h3>
                    <p className="mt-2 text-sm leading-6 text-frost/62">{text}</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="glass-panel mx-auto grid max-w-7xl gap-6 rounded-lg p-8 md:grid-cols-2 md:p-12">
          <FeatureCard icon={ScrollText} eyebrow="Contracts" title="Rust/WASM direction" text="Smart contracts are planned around deterministic execution, bounded resources, predictable fees, contract events and SDK tooling." />
          <FeatureCard icon={Lock} eyebrow="Privacy boundary" title="Off-chain encrypted payloads" text="Encrypted messages and large private data stay off-chain; the chain stores keys, permissions, commitments and receipts where needed." />
        </div>
      </section>
    </>
  )
}
