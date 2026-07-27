import {
  ArrowUpRight,
  Blocks,
  Cpu,
  Download,
  Github,
  HardDrive,
  MonitorSmartphone,
  ShieldCheck,
  WalletCards,
} from 'lucide-react'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import MainnetCandidateBadge from '../components/ui/MainnetCandidateBadge.jsx'

const repository = 'https://github.com/zedkode/Alvenqis-Network'
const releases = `${repository}/releases`

const productModules = [
  {
    id: 'wallet',
    icon: WalletCards,
    eyebrow: 'Wallet',
    title: 'Wallet operations',
    text: 'Create or import an encrypted wallet, inspect balances and prepare signed ALVE transfers without exposing secrets to the website.',
  },
  {
    id: 'mining',
    icon: Cpu,
    eyebrow: 'Mining',
    title: 'Solo and TLS Stratum',
    text: 'Configure local solo mining or a TLS Stratum pool from one control surface with explicit connectivity and worker feedback.',
  },
  {
    id: 'node',
    icon: HardDrive,
    eyebrow: 'Node',
    title: 'Local node control',
    text: 'Manage the bundled node, RPC selection, synchronization state and diagnostics from the same application.',
  },
  {
    id: 'explorer',
    icon: Blocks,
    eyebrow: 'Explorer',
    title: 'Desktop telemetry',
    text: 'Inspect local chain and mining telemetry in-app, then open the dedicated public explorer for shareable block and transaction URLs.',
  },
]

export default function DesktopPage() {
  return (
    <>
      <PageHero
        eyebrow="Alvenqis Desktop V2"
        title="Wallet, mining and node control belong in one secure desktop product."
        text="The website no longer presents Wallet and Mining as disconnected products. Alvenqis Desktop composes both workflows with local node control, diagnostics and explorer telemetry."
      >
        <div className="flex flex-wrap items-center gap-3">
          <MainnetCandidateBadge source="product" />
          <span className="rounded-full border border-gold/30 bg-gold/10 px-4 py-2 font-mono text-xs uppercase tracking-[0.18em] text-gold">
            Public binaries pending verified release
          </span>
        </div>
      </PageHero>

      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="One control center"
            title="A coherent desktop workflow instead of fragmented web promises."
            text="Private keys, mining credentials and local process controls remain in the desktop application. The public website explains the product; it does not imitate sensitive wallet or miner actions."
          />
          <div className="grid gap-4 md:grid-cols-2">
            {productModules.map(({ id, ...module }) => (
              <div id={id} key={id} className="scroll-mt-28">
                <FeatureCard {...module} />
              </div>
            ))}
          </div>
        </div>
      </section>

      <section id="downloads" className="px-5 py-20">
        <div className="mx-auto grid max-w-7xl gap-6 lg:grid-cols-[1.15fr_0.85fr]">
          <article className="glass-panel rounded-lg p-8">
            <div className="flex items-center justify-between gap-4">
              <MonitorSmartphone className="text-ionHot" size={32} />
              <span className="rounded-full border border-line px-3 py-1 font-mono text-xs text-frost/60">
                Windows + Linux targets
              </span>
            </div>
            <h2 className="mt-8 text-4xl font-black text-white">Release channel</h2>
            <p className="mt-4 max-w-2xl leading-8 text-frost/66">
              The canonical GitHub repository currently has no published release assets. No installer URL is advertised until signed binaries, checksums and release notes exist on the official release page.
            </p>
            <div className="mt-8 flex flex-wrap gap-3">
              <a href={releases} className="inline-flex items-center gap-2 rounded-full bg-ionHot px-5 py-3 font-black text-void transition hover:brightness-110">
                <Download size={18} /> Check official releases
              </a>
              <a href={repository} className="inline-flex items-center gap-2 rounded-full border border-line px-5 py-3 font-black text-frost transition hover:border-ionSoft/50">
                <Github size={18} /> Inspect source
              </a>
            </div>
          </article>

          <article className="rounded-lg border border-gold/25 bg-gold/[0.06] p-8">
            <ShieldCheck className="text-gold" size={30} />
            <h2 className="mt-7 text-2xl font-black text-white">Release trust gate</h2>
            <div className="mt-5 space-y-3 text-sm leading-7 text-frost/68">
              <p>1. Download only from the canonical GitHub release page.</p>
              <p>2. Verify the published SHA-256 checksum.</p>
              <p>3. Confirm the installer signature before execution.</p>
              <p>4. Treat missing artifacts as unavailable—not as an invitation to use older builds.</p>
            </div>
            <a href="/status" className="mt-7 inline-flex items-center gap-2 font-bold text-gold">
              Review build status <ArrowUpRight size={17} />
            </a>
          </article>
        </div>
      </section>
    </>
  )
}
