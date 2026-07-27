import { motion } from 'framer-motion'

export const fadeUp = {
  hidden: { opacity: 0, y: 22 },
  visible: { opacity: 1, y: 0 },
}

export function Eyebrow({ children }) {
  return <p className="text-sm font-bold uppercase tracking-[0.3em] text-ionSoft/70">{children}</p>
}

export function SectionHeader({ eyebrow, title, text, align = 'center' }) {
  return (
    <motion.div
      initial="hidden"
      whileInView="visible"
      viewport={{ once: true, margin: '-70px' }}
      variants={fadeUp}
      transition={{ duration: 0.55 }}
      className={`mb-12 ${align === 'center' ? 'mx-auto max-w-3xl text-center' : 'max-w-3xl'}`}
    >
      <Eyebrow>{eyebrow}</Eyebrow>
      <h2 className="mt-4 text-balance text-4xl font-black tracking-tight text-white sm:text-6xl">{title}</h2>
      {text && <p className="mt-5 text-lg leading-8 text-frost/65">{text}</p>}
    </motion.div>
  )
}

export function PageHero({ eyebrow, title, text, children }) {
  return (
    <section className="relative overflow-hidden px-5 pb-16 pt-36">
      <div className="absolute inset-0 bg-grid bg-[length:46px_46px] opacity-30" />
      <div className="absolute inset-x-0 top-24 h-px aurora-line" />
      <motion.div
        initial="hidden"
        animate="visible"
        transition={{ staggerChildren: 0.1 }}
        className="relative mx-auto max-w-7xl"
      >
        <motion.div variants={fadeUp} className="max-w-4xl">
          <Eyebrow>{eyebrow}</Eyebrow>
          <h1 className="mt-5 text-balance text-5xl font-black leading-[0.95] tracking-tight text-white sm:text-7xl">{title}</h1>
          <p className="mt-7 max-w-3xl text-lg leading-8 text-frost/68">{text}</p>
        </motion.div>
        {children && <motion.div variants={fadeUp} className="mt-10">{children}</motion.div>}
      </motion.div>
    </section>
  )
}

export function FeatureCard({ title, text, eyebrow, icon: Icon }) {
  return (
    <motion.article
      variants={fadeUp}
      whileHover={{ y: -5 }}
      className="glass-panel rounded-lg p-6"
    >
      <div className="mb-8 flex items-center justify-between gap-4">
        <span className="rounded-full border border-ionSoft/20 px-3 py-1 text-xs uppercase tracking-[0.18em] text-ionSoft/70">{eyebrow}</span>
        {Icon && <Icon className="text-ionHot" size={24} />}
      </div>
      <h3 className="text-2xl font-black text-white">{title}</h3>
      <p className="mt-4 leading-7 text-frost/66">{text}</p>
    </motion.article>
  )
}

export function MetricCard({ label, value }) {
  return (
    <div className="rounded-lg border border-line bg-white/[0.035] p-4">
      <div className="text-xs uppercase tracking-[0.22em] text-ionSoft/60">{label}</div>
      <div className="mt-2 text-xl font-semibold text-frost">{value}</div>
    </div>
  )
}
