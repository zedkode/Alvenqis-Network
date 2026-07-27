export default function Logo({ compact = false }) {
  return (
    <span className="inline-flex items-center gap-3">
      <span className="relative grid h-11 w-11 place-items-center overflow-hidden rounded-lg border border-ionSoft/30 bg-[linear-gradient(145deg,rgba(56,189,248,0.18),rgba(139,92,246,0.16),rgba(5,6,13,0.74))] shadow-ion">
        <svg viewBox="0 0 64 64" aria-hidden="true" className="h-8 w-8">
          <defs>
            <linearGradient id="alvenqis-mark" x1="10" x2="54" y1="8" y2="56">
              <stop stopColor="#7dd3fc" />
              <stop offset="0.55" stopColor="#38bdf8" />
              <stop offset="1" stopColor="#a78bfa" />
            </linearGradient>
          </defs>
          <path d="M10 10h13.6l9.1 33.2L41.9 10H54L39.5 54H25.4L10 10Z" fill="url(#alvenqis-mark)" />
          <path d="M17.4 10h37.1L51.2 20H20.9L17.4 10Z" fill="#eef7ff" fillOpacity="0.9" />
          <path d="M27.9 54 38.8 25.6l5.6 10.3L39.5 54H27.9Z" fill="#11152b" fillOpacity="0.78" />
        </svg>
        <span className="absolute inset-x-1 top-1 h-px bg-ionHot/70" />
        <span className="absolute bottom-1 right-1 h-2 w-2 rounded-full bg-plasma shadow-plasma" />
      </span>
      {!compact && (
        <span>
          <span className="block text-sm font-black tracking-[0.28em] text-white">ALVENQIS</span>
          <span className="block text-xs font-medium tracking-[0.16em] text-ionSoft/70">NETWORK / ALVE</span>
        </span>
      )}
    </span>
  )
}
