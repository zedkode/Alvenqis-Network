import { KeyRound, Layers, Lock, PlugZap, Send, Wallet } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import VisualPanel from '../components/ui/VisualPanel.jsx'
import { useContent } from '../hooks/useContent.js'

export default function WalletPage() {
  const { content } = useContent('wallet')
  const walletFeatures = content.walletFeatures || []
  const icons = [KeyRound, Send, Layers, PlugZap, Lock, Wallet]
  return (
    <>
      <PageHero
        eyebrow="Wallet"
        title="Candidate wallet flows with explicit production boundaries."
        text="The Tauri Control Center and CLI implement ALVE wallet flows. Recovery, signed-package and broader security gates remain before production approval."
      />
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-8 lg:grid-cols-[0.95fr_1.05fr] lg:items-center">
          <VisualPanel variant="wallet" kicker="Wallet visual" title="Keys, ALVE, assets and Passport proofs." />
          <div>
            <SectionHeader align="left" eyebrow="Wallet states" title="Implemented ALVE basics; planned ownership products." text="Create/import and ALVE transfer flows exist in candidate clients. Assets, Passport and dApp connections remain unavailable until their protocol layers exist." />
            <div className="flex flex-wrap gap-3">
              {['Create/import: Candidate', 'ALVE transfers: Candidate', 'dApp connect: Planned'].map((item) => (
                <span key={item} className="rounded-full border border-line bg-white/[0.035] px-4 py-2 text-sm font-bold text-frost/70">{item}</span>
              ))}
            </div>
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Wallet features" title="ALVE now; ownership products only when implemented." text="Candidate clients handle ALVE accounts and transfers. Assets, licenses, Passport, app access and marketplace actions remain planned." />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.08 }} className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {walletFeatures.map(([title, text], index) => (
              <FeatureCard key={title} icon={icons[index]} eyebrow="Wallet" title={title} text={text} />
            ))}
          </motion.div>
        </div>
      </section>
    </>
  )
}
