import { motion, useMotionValue, useSpring } from 'framer-motion'
import { useEffect } from 'react'

export default function CursorGlow() {
  const rawX = useMotionValue(-200)
  const rawY = useMotionValue(-200)
  const x = useSpring(rawX, { stiffness: 120, damping: 24, mass: 0.25 })
  const y = useSpring(rawY, { stiffness: 120, damping: 24, mass: 0.25 })

  useEffect(() => {
    const finePointer = window.matchMedia('(pointer: fine)').matches
    if (!finePointer) return undefined

    const move = (event) => {
      rawX.set(event.clientX - 180)
      rawY.set(event.clientY - 180)
    }

    window.addEventListener('pointermove', move)
    return () => window.removeEventListener('pointermove', move)
  }, [rawX, rawY])

  return (
    <motion.div
      className="pointer-events-none fixed z-[60] hidden h-[360px] w-[360px] rounded-full bg-[radial-gradient(circle,rgba(56,189,248,0.12),rgba(139,92,246,0.07)_36%,transparent_68%)] blur-xl lg:block"
      style={{ x, y }}
    />
  )
}
