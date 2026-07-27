import { BadgeCheck, Fingerprint, KeyRound, Lock, ShieldCheck, Sparkles } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import { useContent } from '../hooks/useContent.js'

export default function PassportPage() {
  const { content } = useContent('passport')
  const passportUseCases = content.passportUseCases || []
  return (
    <>
      <PageHero
        eyebrow="Alvenqis Passport"
        title="A proof layer for identity, access, ownership and reputation."
        text="Alvenqis Passport is planned as a human-friendly proof surface for licenses, achievements, creator drops, authenticity and app access without forcing public KYC."
      >
        <div className="flex flex-col gap-3 sm:flex-row">
          <span className="inline-flex items-center justify-center gap-2 rounded-full bg-ionHot px-6 py-3 font-bold text-void shadow-ion">
            Create Passport <span className="rounded-full bg-void/12 px-2 py-0.5 text-xs">Coming Soon</span>
          </span>
          <a href="/ecosystem" className="inline-flex items-center justify-center rounded-full border border-line px-6 py-3 font-bold text-frost transition hover:border-ionSoft/50 hover:bg-white/[0.04]">
            Learn about product layer
          </a>
        </div>
      </PageHero>

      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Proof model"
            title="Public proofs, private details."
            text="The website presents Passport as a planned layer: wallet-linked but not wallet-only, selective visibility, revocable records, expired access and app-specific proof surfaces."
          />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.1 }} className="grid gap-4 md:grid-cols-3">
            {[
              [Fingerprint, 'Identity commitments', 'Anchor proof commitments and public keys without forcing private identity data on-chain.'],
              [KeyRound, 'Access records', 'Represent app access, licenses, gated files and digital product ownership.'],
              [ShieldCheck, 'Reputation proofs', 'Show reputation, supporter or developer proofs without exposing a full private profile.'],
            ].map(([icon, title, text]) => (
              <FeatureCard key={title} icon={icon} eyebrow="Passport" title={title} text={text} />
            ))}
          </motion.div>
        </div>
      </section>

      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-10 lg:grid-cols-[0.82fr_1.18fr] lg:items-center">
          <div className="glass-panel rounded-lg p-8">
            <Sparkles className="text-ionHot" />
            <h2 className="mt-6 text-4xl font-black text-white">Passport is the product story.</h2>
            <p className="mt-5 leading-8 text-frost/65">
              For a crypto project like Alvenqis, Passport gives normal users a reason to care: proof of ownership, proof of access, proof of authenticity and proof of progress.
            </p>
            <div className="mt-7 inline-flex rounded-full border border-ionSoft/25 px-4 py-2 text-sm font-bold text-ionSoft">
              Connect Wallet: Coming Soon
            </div>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            {passportUseCases.map(([title, text]) => (
              <div key={title} className="rounded-lg border border-line bg-white/[0.035] p-5">
                <BadgeCheck className="text-ionHot" size={20} />
                <h3 className="mt-4 font-black text-white">{title}</h3>
                <p className="mt-3 text-sm leading-6 text-frost/62">{text}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="px-5 py-20">
        <div className="glass-panel mx-auto max-w-7xl rounded-lg p-8 md:p-12">
          <Lock className="text-violetCore" />
          <h2 className="mt-6 text-4xl font-black text-white">No mandatory public KYC claim.</h2>
          <p className="mt-5 max-w-3xl leading-8 text-frost/65">
            Passport should not expose private identity by default. The intended direction is selective visibility and app-specific proofs, with large private data kept off-chain.
          </p>
        </div>
      </section>
    </>
  )
}
