import { Download, Github, MonitorDown, ShieldCheck } from 'lucide-react'
import { FeatureCard, PageHero, SectionHeader } from '../components/ui/PageShell.jsx'
import MainnetCandidateBadge from '../components/ui/MainnetCandidateBadge.jsx'

const repository = 'https://github.com/zedkode/Alvenqis-Network'

const releases = [
  {
    name: 'Alvenqis Control Center V2',
    version: '2.1.0-candidate.1',
    href: `${repository}/releases/tag/desktop-v2.1.0-candidate.1`,
    detail: 'Latest redesigned desktop application for Windows. Includes wallet, node control, explorer telemetry and CUDA-only TLS Stratum mining.',
  },
  {
    name: 'Alvenqis Control Center V1',
    version: '1.1.0-candidate.1',
    href: `${repository}/releases/tag/desktop-v1.1.0-candidate.1`,
    detail: 'Maintained classic desktop application for Windows, updated to the secure TLS Stratum transport and canonical GitHub release channel.',
  },
]

export default function DownloadsPage() {
  return (
    <>
      <PageHero
        eyebrow="Downloads"
        title="Verified Alvenqis desktop builds."
        text="Choose the current V2 experience or the maintained V1 channel. These are Mainnet Candidate builds and Windows may warn until Authenticode signing is enabled."
      >
        <MainnetCandidateBadge source="release" />
      </PageHero>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader
            eyebrow="Windows applications"
            title="Separate release channels."
            text="V1 and V2 assets cannot be mixed. Every release includes SHA-256 checksums for its installer and portable package."
          />
          <div className="grid gap-5 lg:grid-cols-2">
            {releases.map((release) => (
              <article key={release.name} className="glass-panel rounded-lg p-7">
                <div className="flex items-start justify-between gap-5">
                  <MonitorDown className="text-ionHot" size={30} />
                  <span className="rounded-full border border-ionSoft/20 px-3 py-1 font-mono text-xs text-ionSoft/80">
                    {release.version}
                  </span>
                </div>
                <h2 className="mt-8 text-3xl font-black text-white">{release.name}</h2>
                <p className="mt-4 leading-7 text-frost/66">{release.detail}</p>
                <a
                  href={release.href}
                  className="mt-8 inline-flex items-center gap-2 rounded-full bg-ionHot px-5 py-3 font-black text-void transition hover:brightness-110"
                >
                  <Download size={18} /> Open verified release
                </a>
              </article>
            ))}
          </div>
        </div>
      </section>
      <section className="px-5 py-20">
        <div className="mx-auto max-w-7xl">
          <SectionHeader eyebrow="Release safety" title="Verify before running." text="The release page is the source of truth for build status and checksums." />
          <div className="grid gap-4 md:grid-cols-3">
            <FeatureCard icon={ShieldCheck} eyebrow="Integrity" title="SHA-256" text="Compare every downloaded file against the release SHA256SUMS manifest." />
            <FeatureCard icon={Github} eyebrow="Source" title="Canonical repository" text="Only downloads published by zedkode/Alvenqis-Network are official." />
            <FeatureCard icon={MonitorDown} eyebrow="Platform" title="Windows now" text="Linux packaging is intentionally deferred; no stale Linux artifact is advertised." />
          </div>
        </div>
      </section>
    </>
  )
}
