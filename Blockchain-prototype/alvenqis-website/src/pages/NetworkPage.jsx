import { Blocks, Cpu, Fingerprint } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import { useContent } from '../hooks/useContent.js'

export default function NetworkPage() {
  const { content } = useContent('network')
  const productLayers = content.productLayers || []
  const icons = [Cpu, Blocks, Fingerprint]
  return (
    <>
      <PageHero
        eyebrow="Network architecture"
        title="A mineable Layer 1 split into base, execution and product layers."
        text="Alvenqis is positioned as a Rust-based PoW-first network with an upgrade path toward energy-aware mining research, while keeping large payloads off-chain."
      />
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Layer model"
            title="Clear ownership boundaries."
            text="Each layer has a separate job, which makes it easier to build core protocol, contracts and user-facing products without overloading the chain."
          />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.12 }} className="grid gap-4 md:grid-cols-3">
            {productLayers.map((layer, index) => (
              <FeatureCard key={layer.title} icon={icons[index]} {...layer} />
            ))}
          </motion.div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-5 lg:grid-cols-2">
          {[
            ['On-chain', ['ALVE transfers', 'Fees and settlement', 'Smart contract state', 'Native assets', 'NFT ownership', 'Passport commitments', 'File hashes', 'Marketplace settlement']],
            ['Off-chain', ['Large files', 'NFT images', 'Game assets', 'Encrypted messages', 'Storage blobs', 'Private profile data', 'Chat history', 'Large metadata']],
          ].map(([title, items]) => (
            <div key={title} className="glass-panel rounded-lg p-7">
              <h2 className="text-3xl font-black text-white">{title}</h2>
              <div className="mt-6 grid gap-3 sm:grid-cols-2">
                {items.map((item) => (
                  <div key={item} className="rounded-lg border border-line bg-white/[0.03] px-4 py-3 text-sm text-frost/68">{item}</div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </section>
    </>
  )
}
