import { CheckCircle2, CircleDashed, ShieldAlert } from 'lucide-react'
import { PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import MainnetCandidateBadge from '../components/ui/MainnetCandidateBadge.jsx'
import { useNetworkStats } from '../hooks/useNetwork.js'

const statusRows = [
  ['Website shell', 'Ready prototype', 'Implemented as a React/Vite public interface with multipage content and 3D sections.'],
  ['Core chain', 'Ready prototype', 'Account ledger, FiroPoW, fees, P2P and candidate persistence exist; production storage and G4 evidence remain.'],
  ['Wallet', 'Ready prototype', 'CLI and Tauri wallet/keystore flows exist; signing, recovery and package audit gates remain.'],
  ['Explorer', 'Ready prototype', 'Explorer and indexer consume real candidate RPC data; production database/hosting maturity remains.'],
  ['Developers', 'Ready prototype', 'TypeScript/Rust SDKs and examples exist; contract/application tooling remains planned.'],
  ['Mining pool', 'Ready prototype', 'A reachable VarDiff/PPLNS candidate pool exists without production payout signer/storage approval.'],
  ['Public Mainnet', 'Not live', 'G4 independent review, soak, signing, operations and explicit approval are incomplete.'],
]

function StatusIcon({ state }) {
  if (state === 'Ready prototype') return <CheckCircle2 className="text-ionHot" size={20} />
  if (state === 'Preview only') return <CircleDashed className="text-ionSoft" size={20} />
  if (state === 'Not live') return <ShieldAlert className="text-ionSoft" size={20} />
  return <CircleDashed className="text-frost/50" size={20} />
}

export default function StatusPage() {
  const { stats, source } = useNetworkStats()
  const rows = [
    ...statusRows,
    [
      'Mainnet Candidate RPC',
      source === 'rpc' ? 'Ready prototype' : 'Preview only',
      source === 'rpc'
        ? `The Mainnet Candidate RPC is serving chain state at height ${stats.height}. This is not a public Mainnet launch.`
        : 'The Mainnet Candidate RPC is offline. No simulated network data is displayed.',
    ],
  ]

  return (
    <>
      <PageHero
        eyebrow="Status"
        title="Honest public readiness for Alvenqis Network."
        text="This page prevents the website from becoming misleading. It separates design, planned modules and operational infrastructure."
      >
        <MainnetCandidateBadge source={source} />
      </PageHero>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Readiness matrix"
            title="What exists now versus what is planned."
            text="Candidate services may be operational while production and public-launch gates remain incomplete."
          />
          <div className="overflow-hidden rounded-lg border border-line">
            {rows.map(([name, state, note]) => (
              <div key={name} className="grid gap-4 border-b border-line bg-white/[0.025] p-5 last:border-b-0 md:grid-cols-[220px_170px_1fr] md:items-center">
                <div className="font-black text-white">{name}</div>
                <div className="flex items-center gap-2 text-sm font-semibold text-ionSoft/80">
                  <StatusIcon state={state} />
                  {state}
                </div>
                <div className="text-sm leading-6 text-frost/62">{note}</div>
              </div>
            ))}
          </div>
        </div>
      </section>
    </>
  )
}
