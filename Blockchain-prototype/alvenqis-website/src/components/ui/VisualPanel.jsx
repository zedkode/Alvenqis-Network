import MiniOrbitScene from '../three/MiniOrbitScene.jsx'

export default function VisualPanel({ variant = 'core', title, kicker }) {
  return (
    <div className="glass-panel relative min-h-[360px] overflow-hidden rounded-lg p-6">
      <MiniOrbitScene variant={variant} />
      <div className="relative z-10 flex h-full min-h-[312px] flex-col justify-between">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.26em] text-ionSoft/80">{kicker}</p>
          <h3 className="mt-3 max-w-xs text-3xl font-black text-white">{title}</h3>
        </div>
        <div className="grid grid-cols-3 gap-2">
          {['Proof', 'State', 'UX'].map((item) => (
            <span key={item} className="rounded-full border border-line bg-void/45 px-3 py-1 text-center text-xs text-frost/55">
              {item}
            </span>
          ))}
        </div>
      </div>
    </div>
  )
}
