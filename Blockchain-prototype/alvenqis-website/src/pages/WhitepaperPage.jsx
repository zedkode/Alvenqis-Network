import { BookOpen, Layers3, Rocket, ShieldAlert } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import { useContent } from '../hooks/useContent.js'

export default function WhitepaperPage() {
  const { content } = useContent('whitepaper')
  const whitepaperSections = content.whitepaperSections || []
  return (
    <>
      <PageHero
        eyebrow="Whitepaper preview"
        title="A cinematic entry point for the Alvenqis source of truth."
        text="This page is a website-level whitepaper overview. It summarizes identity, economics, architecture, standards, roadmap and risk without pretending the full network is live."
      >
        <div className="flex flex-col gap-3 sm:flex-row">
          <a href="/docs" className="inline-flex items-center justify-center gap-2 rounded-full bg-ionHot px-6 py-3 font-bold text-void shadow-ion">
            Read docs map <BookOpen size={18} />
          </a>
          <a href="/protocol" className="inline-flex items-center justify-center rounded-full border border-line px-6 py-3 font-bold text-frost transition hover:border-ionSoft/50 hover:bg-white/[0.04]">
            Review open protocol decisions
          </a>
        </div>
      </PageHero>

      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Whitepaper map"
            title="The narrative in six blocks."
            text="This structure can later connect directly to the full whitepaper file or rendered PDF."
          />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.08 }} className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {whitepaperSections.map(([title, text], index) => {
              const icons = [BookOpen, Layers3, Rocket, BookOpen, Rocket, ShieldAlert]
              return <FeatureCard key={title} icon={icons[index]} eyebrow="Section" title={title} text={text} />
            })}
          </motion.div>
        </div>
      </section>

      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <div className="relative overflow-hidden rounded-lg border border-line bg-ink/60 p-8 md:p-12">
            <div className="absolute left-0 top-0 h-px w-full aurora-line" />
            <p className="text-sm font-bold uppercase tracking-[0.3em] text-ionSoft/70">Launch sequence</p>
            <h2 className="mt-4 text-4xl font-black text-white sm:text-6xl">Specs become code. Code becomes a hardened candidate. Evidence enables launch.</h2>
            <div className="mt-10 grid gap-3 md:grid-cols-4">
              {['Source Info', 'Core Chain', 'Mainnet Candidate', 'Security Gates'].map((item, index) => (
                <motion.div
                  key={item}
                  initial={{ opacity: 0, y: 16 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  viewport={{ once: true }}
                  transition={{ delay: index * 0.08 }}
                  className="rounded-lg border border-line bg-void/45 p-5"
                >
                  <div className="text-3xl font-black text-ionHot">0{index + 1}</div>
                  <div className="mt-4 font-black text-white">{item}</div>
                </motion.div>
              ))}
            </div>
          </div>
        </div>
      </section>
    </>
  )
}
