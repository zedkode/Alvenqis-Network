import { motion } from 'framer-motion'
import { ArrowRight, Blocks, ChevronDown, Cpu, Fingerprint, MonitorSmartphone, Orbit, ShieldCheck, Sparkles } from 'lucide-react'
import HeroScene from '../components/HeroScene.jsx'
import { FeatureCard, SectionHeader, fadeUp } from '../components/ui/PageShell.jsx'
import VisualPanel from '../components/ui/VisualPanel.jsx'
import { useContent } from '../hooks/useContent.js'

const heroPills = ['Ticker ALVE', 'PoW first', 'Alvenqis Passport', '60 sec blocks']

function Hero() {
  return (
    <section className="relative flex min-h-screen items-center overflow-hidden px-5 pt-24">
      <div className="absolute inset-0">
        <HeroScene />
      </div>
      <motion.div
        className="pointer-events-none absolute left-0 right-0 z-10 h-8 -translate-y-4 bg-gradient-to-r from-transparent via-ionSoft/15 to-transparent blur-md"
        initial={{ top: '12%', opacity: 0 }}
        animate={{ top: ['12%', '90%', '12%'], opacity: [0, 0.45, 0] }}
        transition={{ duration: 7, repeat: Infinity, ease: 'easeInOut' }}
      />
      <div className="absolute inset-0 bg-grid bg-[length:46px_46px] opacity-30" />
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_68%_45%,transparent_0%,rgba(5,6,13,0.08)_28%,rgba(5,6,13,0.72)_74%),linear-gradient(90deg,rgba(5,6,13,0.96)_0%,rgba(5,6,13,0.74)_40%,rgba(5,6,13,0.18)_100%)]" />
      <div className="absolute inset-x-0 top-24 h-px aurora-line" />

      <div className="relative z-10 mx-auto w-full max-w-7xl pb-20 pt-10">
        <motion.div initial="hidden" animate="visible" transition={{ staggerChildren: 0.12 }} className="max-w-2xl">
          <motion.div variants={fadeUp} className="mb-5 inline-flex items-center gap-2 font-mono text-xs uppercase tracking-[0.22em] text-ionSoft/80">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-ionHot" />
            Alvenqis Network
          </motion.div>
          <motion.h1 variants={fadeUp} className="max-w-3xl text-balance text-4xl font-black leading-[1.02] tracking-tight text-white sm:text-5xl lg:text-6xl">
            Alvenqis <span className="plasma-text">Core</span>
          </motion.h1>
          <motion.p variants={fadeUp} className="mt-5 max-w-xl text-xl leading-snug text-frost/90 sm:text-2xl">
            A mineable Layer 1 direction for apps, games, software licenses and verifiable digital ownership.
          </motion.p>
          <motion.p variants={fadeUp} className="mt-5 max-w-xl text-base leading-8 text-frost/62 sm:text-lg">
            One public portal connects the protocol, the read-only explorer and Alvenqis Desktop—the only product surface for wallet, mining and node control.
          </motion.p>
          <motion.div variants={fadeUp} className="mt-8 flex flex-col gap-3 sm:flex-row">
            <a href="/network" className="group relative inline-flex items-center justify-center gap-2 overflow-hidden rounded-full bg-gradient-to-r from-ionHot to-violetCore px-6 py-3 font-bold text-void shadow-ion transition hover:scale-[1.02]">
              <span className="relative z-10">Enter Alvenqis</span>
              <ArrowRight className="relative z-10" size={18} />
              <span className="absolute inset-0 -translate-x-full skew-x-12 bg-white/25 transition-transform duration-700 ease-out group-hover:translate-x-full" />
            </a>
            <a href="/core" className="inline-flex items-center justify-center rounded-full border border-line px-6 py-3 font-bold text-frost transition hover:border-ionSoft/50 hover:bg-white/[0.04]">
              Core Coming Soon
            </a>
            <a href="/passport" className="inline-flex items-center justify-center rounded-full border border-plasma/35 px-6 py-3 font-bold text-violetCore transition hover:bg-plasma/10">
              Passport Coming Soon
            </a>
          </motion.div>
          <motion.div variants={fadeUp} className="mt-9 flex flex-wrap gap-3">
            {heroPills.map((pill) => (
              <span key={pill} className="rounded-full border border-line bg-white/[0.035] px-4 py-1.5 font-mono text-xs tracking-wide text-frost/54 transition hover:border-ionSoft/35 hover:text-ionHot">
                {pill}
              </span>
            ))}
          </motion.div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, x: 30 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.8, delay: 1.1 }}
          className="absolute bottom-24 right-0 hidden w-80 lg:block"
        >
          <div className="glass-panel rounded-lg p-5">
            <p className="font-mono text-[10px] uppercase tracking-[0.24em] text-ionSoft/80">Ecosystem status</p>
            <div className="mt-4 space-y-3">
              {[
                ['Alvenqis Core', 'building'],
                ['Desktop wallet', 'integrated'],
                ['Passport proofs', 'planned'],
                ['Web explorer', 'read-only'],
              ].map(([item, status]) => (
                <div key={item} className="flex items-center justify-between rounded-lg border border-line bg-void/45 px-3 py-2">
                  <span className="text-sm text-frost/78">{item}</span>
                  <span className="font-mono text-[10px] uppercase tracking-wider text-frost/42">{status}</span>
                </div>
              ))}
            </div>
          </div>
        </motion.div>
      </div>

      <motion.div
        className="absolute bottom-7 left-1/2 z-10 -translate-x-1/2"
        animate={{ y: [0, 8, 0] }}
        transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
      >
        <div className="grid h-9 w-6 place-items-center rounded-full border border-line text-ionSoft/75">
          <ChevronDown size={14} />
        </div>
      </motion.div>
    </section>
  )
}

function CinematicStack() {
  const cards = [
    ['01', 'Protocol Core', 'PoW, blocks, ALVE, state and settlement.', 'lg:mr-16 lg:-rotate-2'],
    ['02', 'Passport Layer', 'Proofs for licenses, access and ownership.', 'lg:ml-12 lg:rotate-2'],
    ['03', 'Product Surface', 'Wallet, explorer, marketplace, docs and status.', 'lg:mr-4 lg:-rotate-1'],
  ]

  return (
    <section className="relative overflow-hidden px-5 py-24 lg:py-32">
      <div className="absolute inset-x-0 top-1/2 h-px aurora-line opacity-70" />
      <div className="mx-auto grid max-w-7xl gap-12 lg:grid-cols-[0.86fr_1.14fr] lg:items-center">
        <div className="relative z-10">
          <p className="text-sm font-bold uppercase tracking-[0.3em] text-ionSoft/70">Visual direction</p>
          <h2 className="mt-4 max-w-2xl text-4xl font-black tracking-tight text-white sm:text-5xl xl:text-6xl">
            Less terminal. More cinematic crypto interface.
          </h2>
          <p className="mt-5 max-w-xl text-lg leading-8 text-frost/65">
            The palette now uses ion cyan, electric blue and violet plasma over deep graphite, matching the more premium 3D direction from the earlier Alvenqis website conversations.
          </p>
        </div>
        <div className="relative z-10 grid gap-5 lg:pl-8">
          {cards.map(([number, title, text, offset], index) => (
            <motion.div
              key={title}
              initial={{ opacity: 0, y: 32 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: index * 0.12, duration: 0.65 }}
              className={`glass-panel w-full rounded-lg p-6 shadow-plasma transition-transform duration-500 hover:rotate-0 lg:max-w-[34rem] ${offset}`}
            >
              <div className="flex items-center justify-between">
                <span className="text-5xl font-black text-ionHot/90">{number}</span>
                <Orbit className="text-violetCore" />
              </div>
              <h3 className="mt-8 text-2xl font-black text-white">{title}</h3>
              <p className="mt-3 leading-7 text-frost/64">{text}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

function ArchitecturePreview({ productLayers }) {
  const icons = [Cpu, Blocks, Fingerprint]
  return (
    <section className="px-5 py-24">
      <div className="mx-auto max-w-7xl">
        <SectionHeader
          eyebrow="Architecture"
          title="Three layers, designed for practical products."
          text="Alvenqis separates settlement, execution and product UX so the website can explain the real system without mixing protocol code with marketing claims."
        />
        <motion.div initial="hidden" whileInView="visible" viewport={{ once: true, margin: '-80px' }} transition={{ staggerChildren: 0.12 }} className="grid gap-4 md:grid-cols-3">
          {productLayers.map((layer, index) => (
            <FeatureCard key={layer.title} icon={icons[index]} {...layer} />
          ))}
        </motion.div>
      </div>
    </section>
  )
}

function ProtocolMatrix({ onChainItems, offChainItems }) {
  return (
    <section className="px-5 py-24">
      <div className="mx-auto grid max-w-7xl gap-8 lg:grid-cols-[0.95fr_1.05fr] lg:items-center">
        <VisualPanel variant="core" kicker="Protocol matrix" title="The chain should store proofs, not heavy payloads." />
        <div>
          <SectionHeader
            align="left"
            eyebrow="Settlement boundary"
            title="On-chain where trust matters. Off-chain where data gets heavy."
            text="This gives the website a real technical story: Alvenqis is not trying to store every file or message directly on-chain."
          />
          <div className="grid gap-4 md:grid-cols-2">
            {[
              ['On-chain', onChainItems],
              ['Off-chain', offChainItems],
            ].map(([title, items]) => (
              <div key={title} className="rounded-lg border border-line bg-white/[0.035] p-5">
                <h3 className="text-2xl font-black text-white">{title}</h3>
                <div className="mt-5 flex flex-wrap gap-2">
                  {items.slice(0, 10).map((item) => (
                    <span key={item} className="rounded-full border border-line bg-void/45 px-3 py-1 text-xs text-frost/60">
                      {item}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}

function ConfirmationStrip({ confirmationModel }) {
  return (
    <section className="px-5 py-20">
      <div className="mx-auto max-w-7xl">
        <SectionHeader
          eyebrow="Confirmation UX"
          title="60-second blocks need a layered UX."
          text="A serious chain website should explain what users see before, during and after final confirmation."
        />
        <div className="grid gap-3 md:grid-cols-4">
          {confirmationModel.map(([label, time, text], index) => (
            <motion.div
              key={label}
              initial={{ opacity: 0, y: 24 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: index * 0.08 }}
              className="glass-panel rounded-lg p-5"
            >
              <div className="text-3xl font-black text-ionHot">{time}</div>
              <h3 className="mt-4 font-black text-white">{label}</h3>
              <p className="mt-3 text-sm leading-6 text-frost/60">{text}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

function ProductGateway() {
  const links = [
    ['Core', '/core', 'Consensus, blocks, state and validation.', Blocks],
    ['Desktop', '/desktop', 'Wallet, mining, node control and local telemetry in one signed application.', MonitorSmartphone],
    ['Tokenomics', '/tokenomics', 'Supply, rewards, halvings and open allocation decisions.', Sparkles],
    ['Explorer', '/explorer', 'Read-only blocks, transactions, addresses, supply and network health.', Blocks],
    ['Passport', '/passport', 'Identity, access, ownership and reputation proofs.', Fingerprint],
    ['Developers', '/developers', 'SDK, contracts, standards and examples.', Orbit],
  ]

  return (
    <section className="px-5 py-24">
      <div className="mx-auto max-w-7xl">
        <SectionHeader
          eyebrow="Product gateway"
          title="More than a landing page: a complete L1 product map."
          text="Wallet and mining are intentionally composed inside Desktop. Public chain inspection remains isolated in the read-only explorer."
        />
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {links.map(([title, href, text, Icon], index) => (
            <motion.a
              key={title}
              href={href}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: index * 0.06 }}
              className="group rounded-lg border border-line bg-ink/65 p-6 transition hover:-translate-y-1 hover:border-ionSoft/35 hover:bg-white/[0.045]"
            >
              <Icon className="text-ionHot transition group-hover:text-violetCore" />
              <h3 className="mt-8 text-2xl font-black text-white">{title}</h3>
              <p className="mt-3 leading-7 text-frost/62">{text}</p>
              <span className="mt-6 inline-flex items-center gap-2 text-sm font-bold text-ionSoft">
                Open page <ArrowRight size={16} />
              </span>
            </motion.a>
          ))}
        </div>
      </div>
    </section>
  )
}

function EcosystemPreview({ ecosystemProducts }) {
  return (
    <section className="px-5 py-24">
      <div className="mx-auto max-w-7xl">
        <SectionHeader
          eyebrow="Ecosystem modules"
          title="Desktop, explorer, indexer, RPC, Passport and SDK."
          text="This is the minimum product layer needed for a modern app/game-friendly Layer 1."
        />
        <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
          {ecosystemProducts.map(([title, text], index) => (
            <motion.div
              key={title}
              initial={{ opacity: 0, scale: 0.96 }}
              whileInView={{ opacity: 1, scale: 1 }}
              viewport={{ once: true }}
              transition={{ delay: index * 0.05 }}
              className="rounded-lg border border-line bg-white/[0.035] p-5"
            >
              <div className="text-xs uppercase tracking-[0.22em] text-ionSoft/60">Module {String(index + 1).padStart(2, '0')}</div>
              <h3 className="mt-4 text-xl font-black text-white">{title}</h3>
              <p className="mt-3 text-sm leading-6 text-frost/60">{text}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

function StandardsPreview({ standards }) {
  return (
    <section className="px-5 py-24">
      <div className="mx-auto grid max-w-7xl gap-10 lg:grid-cols-[0.9fr_1.1fr] lg:items-center">
        <div>
          <p className="text-sm font-bold uppercase tracking-[0.3em] text-ionSoft/70">VRC Standards</p>
          <h2 className="mt-4 text-4xl font-black tracking-tight text-white sm:text-6xl">A standard system for assets, proofs and access.</h2>
          <p className="mt-5 text-lg leading-8 text-frost/65">
            Alvenqis keeps heavy files off-chain and anchors settlement, ownership, permissions, hashes and critical state on-chain.
          </p>
        </div>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {standards.map(([code, label]) => (
            <motion.div key={code} whileHover={{ y: -4, scale: 1.02 }} className="rounded-lg border border-line bg-ink/70 p-5">
              <div className="text-lg font-black text-white">{code}</div>
              <div className="mt-2 text-sm text-frost/55">{label}</div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

function RoadmapPreview({ roadmap }) {
  return (
    <section className="px-5 py-24">
      <div className="mx-auto max-w-7xl">
        <SectionHeader
          eyebrow="Build path"
          title="Protocol first, candidate hardening next, public launch last."
          text="The roadmap is intentionally honest. It does not claim live infrastructure before code, docs and verification exist."
        />
        <div className="overflow-hidden rounded-lg border border-line">
          {roadmap.slice(0, 5).map(([phase, title, status], index) => (
            <motion.div
              key={phase}
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ delay: index * 0.07 }}
              className="grid gap-4 border-b border-line bg-white/[0.025] p-5 last:border-b-0 md:grid-cols-[160px_1fr_130px] md:items-center"
            >
              <span className="font-black text-ionHot">{phase}</span>
              <span className="text-lg font-semibold text-frost">{title}</span>
              <span className="w-fit rounded-full border border-ionSoft/25 px-3 py-1 text-sm text-ionSoft/75">{status}</span>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

function TrustPanel() {
  return (
    <section className="px-5 py-24">
      <div className="glass-panel mx-auto grid max-w-7xl gap-10 rounded-lg p-8 md:p-12 lg:grid-cols-[0.85fr_1.15fr] lg:items-center">
        <div>
          <p className="text-sm font-bold uppercase tracking-[0.3em] text-ionSoft/70">Positioning</p>
          <h2 className="mt-4 text-4xl font-black text-white sm:text-5xl">Premium crypto design, serious engineering language.</h2>
          <p className="mt-5 leading-8 text-frost/65">
            The interface is designed to feel modern and high-end while keeping protocol risk, open decisions and development stage visible.
          </p>
        </div>
        <div className="grid gap-4 md:grid-cols-3">
          {[
            ['Honest status', 'Candidate wallet, explorer and pool are labeled separately from public production Mainnet.'],
            ['Source aligned', 'Content follows Alvenqis Network / ALVE identity and architecture.'],
            ['Live candidate data', 'Current pages consume real candidate RPC data and expose offline states honestly.'],
          ].map(([title, text]) => (
            <div key={title} className="rounded-lg border border-line bg-void/45 p-5">
              <ShieldCheck className="text-ionHot" />
              <h3 className="mt-5 font-black text-white">{title}</h3>
              <p className="mt-3 text-sm leading-6 text-frost/62">{text}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

export default function HomePage() {
  const { content } = useContent('home')

  return (
    <>
      <Hero />
      <CinematicStack />
      <ProductGateway />
      <ArchitecturePreview productLayers={content.productLayers || []} />
      <ProtocolMatrix onChainItems={content.onChainItems || []} offChainItems={content.offChainItems || []} />
      <ConfirmationStrip confirmationModel={content.confirmationModel || []} />
      <EcosystemPreview ecosystemProducts={content.ecosystemProducts || []} />
      <StandardsPreview standards={content.standards || []} />
      <RoadmapPreview roadmap={content.roadmap || []} />
      <TrustPanel />
    </>
  )
}
