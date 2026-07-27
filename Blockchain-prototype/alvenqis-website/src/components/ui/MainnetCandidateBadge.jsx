import { Activity } from 'lucide-react'

export default function MainnetCandidateBadge({ source = 'rpc' }) {
  const isOffline = source !== 'rpc'

  return (
    <div className="inline-flex w-fit items-center gap-2 rounded-full border border-ionSoft/30 bg-ionSoft/10 px-4 py-2 text-sm font-bold text-ionHot">
      <Activity size={16} />
      Mainnet Candidate
      {isOffline && <span className="rounded-full bg-void/45 px-2 py-0.5 text-[10px] uppercase tracking-wider text-frost/58">RPC offline</span>}
    </div>
  )
}
