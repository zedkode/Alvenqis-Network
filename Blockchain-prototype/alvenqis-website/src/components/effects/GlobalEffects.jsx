import { motion, useScroll, useTransform } from 'framer-motion'

export default function GlobalEffects() {
  const { scrollYProgress } = useScroll()
  const y = useTransform(scrollYProgress, [0, 1], ['0%', '28%'])
  const yReverse = useTransform(scrollYProgress, [0, 1], ['0%', '-18%'])
  const rotate = useTransform(scrollYProgress, [0, 1], [0, 18])

  return (
    <div className="pointer-events-none fixed inset-0 z-0 overflow-hidden">
      <motion.div
        style={{ y, rotate }}
        className="absolute -right-28 top-24 h-96 w-96 rounded-full border border-ionSoft/10 bg-ionSoft/10 blur-3xl"
      />
      <motion.div
        style={{ y: yReverse }}
        className="absolute -left-28 top-1/2 h-[34rem] w-[34rem] rounded-full border border-plasma/10 bg-plasma/10 blur-3xl"
      />
      <div className="absolute inset-0 bg-[linear-gradient(120deg,transparent_0%,rgba(125,211,252,0.045)_35%,transparent_70%)]" />
    </div>
  )
}
