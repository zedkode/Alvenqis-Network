import { Code2, Database, FileCode2, Layers3, Server, Workflow } from 'lucide-react'
import { motion } from 'framer-motion'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import VisualPanel from '../components/ui/VisualPanel.jsx'
import { useContent } from '../hooks/useContent.js'

export default function DevelopersPage() {
  const { content } = useContent('developers')
  const developerStack = content.developerStack || []
  const standards = content.standards || []
  const icons = [Server, Code2, FileCode2, Database, Database, Workflow]
  return (
    <>
      <PageHero
        eyebrow="Developers"
        title="A builder surface for apps, games, contracts and digital products."
        text="Developer content makes Alvenqis feel like a platform, not just a coin website. SDKs, contract standards and examples become real as the core matures."
      />
      <section className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-8 lg:grid-cols-[0.95fr_1.05fr] lg:items-center">
          <VisualPanel variant="core" kicker="Developer visual" title="SDKs, contracts, events and app integrations." />
          <div className="grid gap-3 sm:grid-cols-2">
            {standards.slice(0, 6).map(([code, label]) => (
              <div key={code} className="rounded-lg border border-line bg-white/[0.035] p-4">
                <Layers3 className="text-ionHot" size={20} />
                <h3 className="mt-4 font-black text-white">{code}</h3>
                <p className="mt-2 text-sm text-frost/58">{label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Stack" title="The technical stack behind the product vision." text="This makes the site useful to engineers and future contributors." />
          <motion.div initial="hidden" whileInView="visible" viewport={{ once: true }} transition={{ staggerChildren: 0.08 }} className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {developerStack.map(([title, text], index) => (
              <FeatureCard key={title} icon={icons[index]} eyebrow="Stack" title={title} text={text} />
            ))}
          </motion.div>
        </div>
      </section>
    </>
  )
}
