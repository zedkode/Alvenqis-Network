import { useEffect } from 'react'

export default function useSmoothScroll() {
  useEffect(() => {
    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const coarsePointer = window.matchMedia('(pointer: coarse)').matches
    if (reduceMotion || coarsePointer) return undefined

    let current = window.scrollY
    let target = window.scrollY
    let frame = null
    let isAnimating = false

    const clamp = (value) => {
      const max = document.documentElement.scrollHeight - window.innerHeight
      return Math.max(0, Math.min(value, max))
    }

    const tick = () => {
      current += (target - current) * 0.11
      if (Math.abs(target - current) < 0.5) {
        current = target
        isAnimating = false
        frame = null
        window.scrollTo(0, current)
        return
      }
      window.scrollTo(0, current)
      frame = requestAnimationFrame(tick)
    }

    const onWheel = (event) => {
      if (event.ctrlKey || event.metaKey || event.shiftKey) return
      event.preventDefault()
      target = clamp(target + event.deltaY * 0.88)
      if (!isAnimating) {
        current = window.scrollY
        isAnimating = true
        frame = requestAnimationFrame(tick)
      }
    }

    const sync = () => {
      if (!isAnimating) {
        current = window.scrollY
        target = window.scrollY
      }
    }

    window.addEventListener('wheel', onWheel, { passive: false })
    window.addEventListener('resize', sync)
    window.addEventListener('keydown', sync)

    return () => {
      if (frame) cancelAnimationFrame(frame)
      window.removeEventListener('wheel', onWheel)
      window.removeEventListener('resize', sync)
      window.removeEventListener('keydown', sync)
    }
  }, [])
}
