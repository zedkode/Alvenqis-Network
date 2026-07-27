import { Blocks, Cpu, Database, Network, Terminal, Zap } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import VisualPanel from '../components/ui/VisualPanel.jsx'
import { useContent } from '../hooks/useContent.js'

export default function CorePage() {
  const { content } = useContent('core')
  const chainFacts = content.chainFacts || []
  const confirmationModel = content.confirmationModel || []
  const coreModules = content.coreModules || []
  const openDecisions = content.openDecisions || []
  const icons = [Blocks, Zap, Database, Network, Database, Terminal, Terminal, Cpu]
  return (
    <>
      <PageHero
        eyebrow="Alvenqis Core"
        title="The Rust protocol engine behind ALVE, blocks, mining and settlement."
        text="Core is the part that must be proven before any public network claim. The site now separates what is defined from what is still a hard protocol decision."
      />
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-8 lg:grid-cols-[0.95fr_1.05fr] lg:items-center">
          <VisualPanel variant="core" kicker="Core visual" title="Blocks, state, mempool and validation." />
          <div className="grid gap-3 sm:grid-cols-2">
            {chainFacts.map(([label, value, text]) => (
              <div key={label} className="rounded-lg border border-line bg-white/[0.035] p-5">
                <div className="text-xs uppercase tracking-[0.22em] text-ionSoft/60">{label}</div>
                <div className="mt-2 text-2xl font-black text-white">{value}</div>
                <p className="mt-3 text-sm leading-6 text-frost/58">{text}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Core modules" title="What the first real core must contain." text="This is the practical engineering checklist for Phase 1 and Phase 2." />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.08 }} className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            {coreModules.map(([title, text], index) => (
              <FeatureCard key={title} icon={icons[index]} eyebrow="Module" title={title} text={text} />
            ))}
          </motion.div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Confirmation UX" title="60-second blocks need honest confirmation states." text="Wallets and apps should show pending, normal and high-value confirmation levels clearly." />
          <div className="grid gap-3 md:grid-cols-4">
            {confirmationModel.map(([label, time, text]) => (
              <div key={label} className="glass-panel rounded-lg p-5">
                <div className="text-3xl font-black text-ionHot">{time}</div>
                <h3 className="mt-4 font-black text-white">{label}</h3>
                <p className="mt-3 text-sm leading-6 text-frost/60">{text}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Gates" title="Open protocol choices before irreversible implementation." text="These are not weaknesses if documented honestly; they are the correct engineering gates." />
          <div className="grid gap-3 md:grid-cols-2">
            {openDecisions.map(([title, text]) => (
              <div key={title} className="rounded-lg border border-line bg-ink/70 p-5">
                <h3 className="font-black text-white">{title}</h3>
                <p className="mt-2 text-sm leading-6 text-frost/62">{text}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
    </>
  )
}
