import { motion, AnimatePresence } from 'framer-motion'
import { useEffect, useState } from 'react'
import Logo from '../ui/Logo.jsx'

export default function Preloader() {
  const [visible, setVisible] = useState(true)

  useEffect(() => {
    const timer = window.setTimeout(() => setVisible(false), 1200)
    return () => window.clearTimeout(timer)
  }, [])

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          initial={{ opacity: 1 }}
          exit={{ opacity: 0, transition: { duration: 0.55, ease: 'easeInOut' } }}
          className="fixed inset-0 z-[80] grid place-items-center bg-void"
        >
          <div className="absolute inset-0 bg-grid bg-[length:42px_42px] opacity-25" />
          <motion.div
            initial={{ scale: 0.92, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ duration: 0.45 }}
            className="relative text-center"
          >
            <Logo />
            <div className="mt-8 h-1 w-72 overflow-hidden rounded-full bg-white/10">
              <motion.div
                className="h-full bg-gradient-to-r from-ionHot via-ionSoft to-violetCore"
                initial={{ x: '-100%' }}
                animate={{ x: '100%' }}
                transition={{ duration: 1.05, repeat: Infinity, ease: 'easeInOut' }}
              />
            </div>
            <p className="mt-5 font-mono text-xs uppercase tracking-[0.28em] text-frost/45">Initializing Alvenqis interface</p>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
