import { Activity, Boxes, Code2, Fingerprint, Search, Server, Wallet } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import { useContent } from '../hooks/useContent.js'

export default function EcosystemPage() {
  const { content } = useContent('ecosystem')
  const ecosystemProducts = content.ecosystemProducts || []
  const standards = content.standards || []
  const icons = [Wallet, Search, Activity, Server, Fingerprint, Code2]
  return (
    <>
      <PageHero
        eyebrow="Product ecosystem"
        title="Wallet, explorer, Passport, SDK and marketplace as one ownership stack."
        text="Alvenqis's product layer is planned around users, developers, miners and creators, with the chain used for settlement and proofs instead of raw data storage."
      />
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Product modules"
            title="The public interface for a technical network."
            text="These modules are planned surfaces. They become live only after matching code, APIs, docs and status checks exist."
          />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.1 }} className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {ecosystemProducts.map(([title, text], index) => (
              <FeatureCard key={title} icon={icons[index]} eyebrow="Product" title={title} text={text} />
            ))}
          </motion.div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-10 lg:grid-cols-[0.85fr_1.15fr] lg:items-center">
          <div>
            <p className="text-sm font-bold uppercase tracking-[0.3em] text-ionSoft/70">Ownership standards</p>
            <h2 className="mt-4 text-4xl font-black text-white sm:text-6xl">From licenses to game items.</h2>
            <p className="mt-5 leading-8 text-frost/65">
              VRC standards make the ecosystem legible for wallets, explorers, marketplaces, games and app developers.
            </p>
          </div>
          <div className="grid gap-3 sm:grid-cols-3">
            {standards.map(([code, label]) => (
              <div key={code} className="rounded-lg border border-line bg-white/[0.035] p-4">
                <Boxes className="text-ionHot" size={20} />
                <h3 className="mt-4 font-black text-white">{code}</h3>
                <p className="mt-2 text-sm text-frost/58">{label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
    </>
  )
}
