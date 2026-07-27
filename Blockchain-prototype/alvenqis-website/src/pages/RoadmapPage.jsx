import { motion, useScroll, useTransform } from 'framer-motion'
import { useRef } from 'react'
import { PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import { useContent } from '../hooks/useContent.js'

export default function RoadmapPage() {
  const { content } = useContent('roadmap')
  const roadmap = content.roadmap || []
  const timelineRef = useRef(null)
  const { scrollYProgress } = useScroll({ target: timelineRef, offset: ['start center', 'end center'] })
  const scaleY = useTransform(scrollYProgress, [0, 1], [0, 1])

  return (
    <>
      <PageHero
        eyebrow="Roadmap"
        title="A gate-based route from candidate software to public launch."
        text="The roadmap is organized by evidence and approval gates, not by outdated implementation phases or hype milestones."
      />
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Gates G0-G4"
            title="Passing one gate never implies the next."
            text="Candidate code and services exist; production storage, multi-host evidence, independent review, signing and explicit approval still separate them from public Mainnet."
          />
          <div ref={timelineRef} className="relative">
            <div className="absolute left-5 top-0 hidden h-full w-px bg-ionSoft/25 md:block" />
            <motion.div
              className="absolute left-5 top-0 hidden h-full w-px origin-top bg-gradient-to-b from-ionHot to-violetCore shadow-ion md:block"
              style={{ scaleY }}
            />
            <div className="grid gap-4">
              {roadmap.map(([phase, title, status, text], index) => (
                <motion.article
                  key={phase}
                  initial={{ opacity: 0, y: 18 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  viewport={{ once: true }}
                  transition={{ delay: index * 0.05 }}
                  className="glass-panel relative rounded-lg p-6 md:ml-14"
                >
                  <span className="absolute -left-[3.55rem] top-7 hidden h-10 w-10 rounded-full border border-ionSoft/35 bg-void text-center text-sm font-black leading-10 text-ionHot md:block">
                    {index}
                  </span>
                  <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                    <div>
                      <p className="font-black text-ionHot">{phase}</p>
                      <h2 className="mt-1 text-2xl font-black text-white">{title}</h2>
                    </div>
                    <span className="w-fit rounded-full border border-ionSoft/25 px-3 py-1 text-sm text-ionSoft/75">{status}</span>
                  </div>
                  <p className="mt-5 max-w-3xl leading-7 text-frost/64">{text}</p>
                </motion.article>
              ))}
            </div>
          </div>
        </div>
      </section>
    </>
  )
}
