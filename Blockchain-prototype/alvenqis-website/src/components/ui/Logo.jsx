export default function Logo({ compact = false }) {
  return (
    <span className="inline-flex items-center gap-3">
      <span className="relative grid h-11 w-11 place-items-center overflow-hidden rounded-lg border border-ionSoft/30 bg-void shadow-ion">
        <img src="/alvenqis-logo.png" alt="" className="h-full w-full object-cover" />
        <span className="absolute inset-x-1 top-1 h-px bg-ionHot/70" />
        <span className="absolute bottom-1 right-1 h-2 w-2 rounded-full bg-gold shadow-plasma" />
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
