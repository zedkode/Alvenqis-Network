import { HelpCircle } from 'lucide-react'
import { motion } from 'framer-motion'
import { PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import { useContent } from '../hooks/useContent.js'

export default function FaqPage() {
  const { content } = useContent('faq')
  const faqItems = content.faqItems || []
  return (
    <>
      <PageHero
        eyebrow="FAQ"
        title="Clear answers before hype."
        text="A serious crypto website should answer readiness, wallet, mining, storage and protocol questions directly."
      />
      <section className="px-5 py-20">
        <div className="mx-auto max-w-5xl">
          <SectionHeader eyebrow="Questions" title="What visitors need to understand fast." />
          <div className="grid gap-4">
            {faqItems.map(([question, answer], index) => (
              <motion.div
                key={question}
                initial={{ opacity: 0, y: 18 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.05 }}
                className="glass-panel rounded-lg p-6"
              >
                <div className="flex gap-4">
                  <HelpCircle className="mt-1 shrink-0 text-ionHot" />
                  <div>
                    <h2 className="text-xl font-black text-white">{question}</h2>
                    <p className="mt-3 leading-7 text-frost/64">{answer}</p>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>
    </>
  )
}
