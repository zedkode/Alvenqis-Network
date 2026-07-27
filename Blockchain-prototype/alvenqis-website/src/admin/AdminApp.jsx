import { useEffect, useMemo, useState } from 'react'
import {
  Activity,
  BookOpen,
  Database,
  FileText,
  Gauge,
  HelpCircle,
  LayoutDashboard,
  LogOut,
  Network,
  Plus,
  Shield,
  Users,
} from 'lucide-react'
import { Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { adminFetch, devBypassEnabled, getAccessToken, getDevUser, login, logout, refreshSession, setDevUser } from './adminApi.js'
import Logo from '../components/ui/Logo.jsx'

const navItems = [
  ['Dashboard', 'dashboard', LayoutDashboard],
  ['Users', 'users', Users],
  ['Content', 'content', FileText],
  ['Network Params', 'network', Network],
  ['Roadmap', 'roadmap', BookOpen],
  ['FAQ', 'faq', HelpCircle],
  ['Audit Log', 'audit', Shield],
]

const roles = ['superadmin', 'content_editor', 'network_operator']

function pathToSection(path) {
  const [, , section] = path.split('/')
  return section || 'dashboard'
}

function navigate(section) {
  window.history.pushState({}, '', section === 'dashboard' ? '/admin' : `/admin/${section}`)
  window.dispatchEvent(new PopStateEvent('popstate'))
}

function parseJsonValue(value) {
  if (value === 'true') return true
  if (value === 'false') return false
  if (value !== '' && !Number.isNaN(Number(value))) return Number(value)

  try {
    return JSON.parse(value)
  } catch {
    return value
  }
}

function stringify(value) {
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2)
}

function AdminCard({ title, value, hint, icon: Icon }) {
  return (
    <div className="rounded-lg border border-line bg-white/[0.035] p-5">
      <div className="flex items-center justify-between">
        <span className="text-xs uppercase tracking-[0.22em] text-ionSoft/60">{title}</span>
        {Icon && <Icon className="text-ionHot" size={20} />}
      </div>
      <div className="mt-4 text-3xl font-black text-white">{value}</div>
      {hint && <p className="mt-2 text-sm text-frost/55">{hint}</p>}
    </div>
  )
}

function LoginScreen({ onLogin }) {
  const [email, setEmail] = useState('admin@alvenqis.network')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function submit(event) {
    event.preventDefault()
    setLoading(true)
    setError('')

    try {
      const user = await login(email, password)
      onLogin(user)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  function enterDevMode() {
    onLogin(setDevUser(true))
  }

  return (
    <div className="grid min-h-screen place-items-center bg-void px-5 text-frost">
      <form onSubmit={submit} className="glass-panel w-full max-w-md rounded-lg p-7">
        <Logo />
        <p className="mt-8 text-sm font-bold uppercase tracking-[0.26em] text-ionSoft/70">Admin Panel</p>
        <h1 className="mt-3 text-4xl font-black text-white">Sign in to Alvenqis Ops.</h1>
        <div className="mt-8 grid gap-4">
          <input className="rounded-lg border border-line bg-void/70 px-4 py-3 text-sm outline-none focus:border-ionSoft" value={email} onChange={(event) => setEmail(event.target.value)} placeholder="Email" />
          <input className="rounded-lg border border-line bg-void/70 px-4 py-3 text-sm outline-none focus:border-ionSoft" value={password} onChange={(event) => setPassword(event.target.value)} placeholder="Password" type="password" />
          {error && <div className="rounded-lg border border-red-400/30 bg-red-400/10 px-4 py-3 text-sm text-red-200">{error}</div>}
          <button className="rounded-full bg-ionHot px-5 py-3 font-black text-void shadow-ion disabled:opacity-50" disabled={loading}>
            {loading ? 'Signing in...' : 'Login'}
          </button>
          {devBypassEnabled && (
            <button type="button" onClick={enterDevMode} className="rounded-full border border-ionSoft/30 px-5 py-3 font-black text-ionHot transition hover:bg-ionSoft/10">
              Enter dev panel without database
            </button>
          )}
          {devBypassEnabled && (
            <p className="text-xs leading-5 text-frost/48">
              Development only. This opens the admin UI without PostgreSQL/backend. Live data and saves still need the API.
            </p>
          )}
        </div>
      </form>
    </div>
  )
}

function Dashboard() {
  const [data, setData] = useState(null)
  const [error, setError] = useState('')

  useEffect(() => {
    adminFetch('/api/admin/dashboard').then(setData).catch((err) => setError(err.message))
  }, [])

  const kpis = data?.kpis || {}

  return (
    <AdminSection title="Dashboard" text="Operational overview for the Mainnet Candidate, CMS and admin activity." error={error}>
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <AdminCard title="Candidate height" value={kpis.candidateHeight ?? 'offline'} icon={Database} />
        <AdminCard title="Active users" value={kpis.activeUsers ?? '-'} icon={Users} />
        <AdminCard title="Content blocks" value={kpis.contentBlocks ?? '-'} icon={FileText} />
        <AdminCard title="Last login" value={kpis.lastLogin?.email || '-'} hint={kpis.lastLogin?.at || 'No login yet'} icon={Activity} />
      </div>
      <Panel title="Latest admin activity">
        <div className="grid gap-2">
          {(data?.latestAuditLogs || []).map((item) => (
            <div key={item.id} className="grid gap-2 rounded-lg border border-line bg-void/45 p-4 md:grid-cols-[190px_1fr_180px]">
              <span className="font-mono text-xs text-ionHot">{item.action}</span>
              <span className="text-sm text-frost/70">{item.entity}:{item.entityId || '-'}</span>
              <span className="text-xs text-frost/45">{item.createdAt}</span>
            </div>
          ))}
        </div>
      </Panel>
    </AdminSection>
  )
}

function UsersModule() {
  const [items, setItems] = useState([])
  const [form, setForm] = useState({ email: '', password: '', role: 'content_editor', isActive: true })
  const [error, setError] = useState('')

  const load = () => adminFetch('/api/admin/users').then((payload) => setItems(payload.items || [])).catch((err) => setError(err.message))
  useEffect(load, [])

  async function create(event) {
    event.preventDefault()
    await adminFetch('/api/admin/users', { method: 'POST', body: JSON.stringify(form) })
    setForm({ email: '', password: '', role: 'content_editor', isActive: true })
    load()
  }

  async function update(id, patch) {
    await adminFetch(`/api/admin/users/${id}`, { method: 'PUT', body: JSON.stringify(patch) })
    load()
  }

  return (
    <AdminSection title="Users & Permissions" text="Create users, assign roles and deactivate access." error={error}>
      <Panel title="Create user">
        <form onSubmit={create} className="grid gap-3 lg:grid-cols-[1fr_1fr_220px_140px]">
          <input className="admin-input" placeholder="email" value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} />
          <input className="admin-input" placeholder="password" type="password" value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} />
          <select className="admin-input" value={form.role} onChange={(e) => setForm({ ...form, role: e.target.value })}>{roles.map((role) => <option key={role}>{role}</option>)}</select>
          <button className="admin-button"><Plus size={16} /> Create</button>
        </form>
      </Panel>
      <Panel title="Users">
        <div className="grid gap-3">
          {items.map((user) => (
            <div key={user.id} className="grid gap-3 rounded-lg border border-line bg-void/45 p-4 lg:grid-cols-[1fr_220px_140px_140px] lg:items-center">
              <div>
                <div className="font-black text-white">{user.email}</div>
                <div className="text-xs text-frost/45">Last login: {user.lastLogin || 'never'}</div>
              </div>
              <select className="admin-input" defaultValue={user.role} onChange={(e) => update(user.id, { role: e.target.value })}>{roles.map((role) => <option key={role}>{role}</option>)}</select>
              <button className="admin-button secondary" onClick={() => update(user.id, { isActive: !user.isActive })}>{user.isActive ? 'Deactivate' : 'Activate'}</button>
              <span className={`rounded-full px-3 py-1 text-center text-xs font-bold ${user.isActive ? 'bg-ionSoft/10 text-ionHot' : 'bg-red-400/10 text-red-200'}`}>{user.isActive ? 'Active' : 'Disabled'}</span>
            </div>
          ))}
        </div>
      </Panel>
    </AdminSection>
  )
}

function ContentModule() {
  const [items, setItems] = useState([])
  const [form, setForm] = useState({ pageSlug: 'home', sectionKey: '', lang: 'en', contentJson: '{}' })
  const [error, setError] = useState('')

  const load = () => adminFetch('/api/admin/content').then((payload) => setItems(payload.items || [])).catch((err) => setError(err.message))
  useEffect(load, [])

  async function create(event) {
    event.preventDefault()
    await adminFetch('/api/admin/content', {
      method: 'POST',
      body: JSON.stringify({ ...form, contentJson: parseJsonValue(form.contentJson) }),
    })
    setForm({ pageSlug: 'home', sectionKey: '', lang: 'en', contentJson: '{}' })
    load()
  }

  async function update(item, contentJson) {
    await adminFetch(`/api/admin/content/${item.id}`, {
      method: 'PUT',
      body: JSON.stringify({ contentJson: parseJsonValue(contentJson) }),
    })
    load()
  }

  return (
    <AdminSection title="Content Editor" text="Edit CMS content blocks and preview the public site." error={error}>
      <Panel title="Create content block">
        <form onSubmit={create} className="grid gap-3 lg:grid-cols-[160px_1fr_100px_160px]">
          <input className="admin-input" value={form.pageSlug} onChange={(e) => setForm({ ...form, pageSlug: e.target.value })} placeholder="page slug" />
          <input className="admin-input" value={form.sectionKey} onChange={(e) => setForm({ ...form, sectionKey: e.target.value })} placeholder="section key" />
          <input className="admin-input" value={form.lang} onChange={(e) => setForm({ ...form, lang: e.target.value })} placeholder="lang" />
          <button className="admin-button"><Plus size={16} /> Create</button>
          <textarea className="admin-input min-h-32 lg:col-span-4" value={form.contentJson} onChange={(e) => setForm({ ...form, contentJson: e.target.value })} />
        </form>
      </Panel>
      <div className="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
        <Panel title="Editable blocks">
          <div className="grid max-h-[720px] gap-3 overflow-auto pr-1">
            {items.map((item) => (
              <ContentBlockEditor key={item.id} item={item} onSave={update} />
            ))}
          </div>
        </Panel>
        <Panel title="Live preview">
          <iframe title="Public preview" src="/" className="h-[720px] w-full rounded-lg border border-line bg-void" />
        </Panel>
      </div>
    </AdminSection>
  )
}

function ContentBlockEditor({ item, onSave }) {
  const [value, setValue] = useState(stringify(item.contentJson))

  return (
    <div className="rounded-lg border border-line bg-void/45 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="font-black text-white">{item.pageSlug}.{item.sectionKey}</div>
          <div className="text-xs text-frost/45">lang {item.lang} - published</div>
        </div>
        <button className="admin-button secondary" onClick={() => onSave(item, value)}>Save</button>
      </div>
      <textarea className="admin-input mt-3 min-h-36 font-mono text-xs" value={value} onChange={(e) => setValue(e.target.value)} />
    </div>
  )
}

function NetworkParamsModule() {
  const [items, setItems] = useState([])
  const [error, setError] = useState('')
  const params = useMemo(() => Object.fromEntries(items.map((item) => [item.key, item.value])), [items])
  const chartData = useMemo(() => {
    const reward = Number(params.current_reward || 19.02587519)
    const interval = Number(params.halving_interval || 1576800)
    return Array.from({ length: 8 }, (_, index) => ({
      epoch: index,
      block: index * interval,
      reward: reward / (2 ** index),
    }))
  }, [params])

  const load = () => adminFetch('/api/admin/network-params').then((payload) => setItems(payload.items || [])).catch((err) => setError(err.message))
  useEffect(load, [])

  async function update(key, value) {
    const critical = ['max_supply', 'halving_interval', 'current_reward'].includes(key)
    if (critical && !window.confirm(`Confirm critical update for ${key}?`)) return

    await adminFetch(`/api/admin/network-params/${key}`, {
      method: 'PUT',
      body: JSON.stringify({ value: parseJsonValue(value), confirmCritical: critical }),
    })
    load()
  }

  return (
    <AdminSection title="Network Parameters" text="Review Mainnet Candidate parameters. Consensus-critical changes require an explicit release process." error={error}>
      <Panel title="Emission curve">
        <div className="h-72">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={chartData}>
              <XAxis dataKey="epoch" stroke="#7dd3fc" />
              <YAxis stroke="#7dd3fc" />
              <Tooltip contentStyle={{ background: '#080912', border: '1px solid rgba(255,255,255,.12)', color: '#fff' }} />
              <Line type="monotone" dataKey="reward" stroke="#4dfcff" strokeWidth={3} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </Panel>
      <Panel title="Parameters">
        <div className="grid gap-3">
          {items.map((item) => (
            <NetworkParamRow key={item.key} item={item} onSave={update} />
          ))}
        </div>
      </Panel>
    </AdminSection>
  )
}

function NetworkParamRow({ item, onSave }) {
  const [value, setValue] = useState(stringify(item.value))

  return (
    <div className="grid gap-3 rounded-lg border border-line bg-void/45 p-4 lg:grid-cols-[240px_1fr_120px] lg:items-center">
      <div>
        <div className="font-mono text-sm font-black text-ionHot">{item.key}</div>
        <div className="text-xs text-frost/45">Updated {item.updatedAt || '-'}</div>
      </div>
      <input className="admin-input" value={value} onChange={(e) => setValue(e.target.value)} />
      <button className="admin-button secondary" onClick={() => onSave(item.key, value)}>Save</button>
    </div>
  )
}

function RoadmapModule() {
  const [items, setItems] = useState([])
  const [dragId, setDragId] = useState(null)
  const [form, setForm] = useState({ phase: '', title: '', description: '', status: 'planned', order: 0 })
  const [error, setError] = useState('')

  const load = () => adminFetch('/api/admin/roadmap').then((payload) => setItems(payload.items || [])).catch((err) => setError(err.message))
  useEffect(load, [])

  async function create(event) {
    event.preventDefault()
    await adminFetch('/api/admin/roadmap', { method: 'POST', body: JSON.stringify(form) })
    setForm({ phase: '', title: '', description: '', status: 'planned', order: 0 })
    load()
  }

  async function saveOrder(nextItems) {
    setItems(nextItems)
    await Promise.all(nextItems.map((item, index) => adminFetch(`/api/admin/roadmap/${item.id}`, { method: 'PUT', body: JSON.stringify({ order: index }) })))
    load()
  }

  function dropOn(targetId) {
    if (!dragId || dragId === targetId) return
    const source = items.find((item) => item.id === dragId)
    const withoutSource = items.filter((item) => item.id !== dragId)
    const targetIndex = withoutSource.findIndex((item) => item.id === targetId)
    const next = [...withoutSource]
    next.splice(targetIndex, 0, source)
    saveOrder(next)
  }

  return (
    <AdminSection title="Roadmap Management" text="Visual CRUD with native drag-and-drop ordering." error={error}>
      <Panel title="Create roadmap item">
        <form onSubmit={create} className="grid gap-3 lg:grid-cols-[140px_1fr_180px_120px]">
          <input className="admin-input" placeholder="Phase" value={form.phase} onChange={(e) => setForm({ ...form, phase: e.target.value })} />
          <input className="admin-input" placeholder="Title" value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />
          <select className="admin-input" value={form.status} onChange={(e) => setForm({ ...form, status: e.target.value })}>{['active', 'next', 'planned', 'research', 'future', 'completed'].map((status) => <option key={status}>{status}</option>)}</select>
          <button className="admin-button"><Plus size={16} /> Add</button>
          <textarea className="admin-input min-h-24 lg:col-span-4" placeholder="Description" value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} />
        </form>
      </Panel>
      <Panel title="Roadmap items">
        <div className="grid gap-3">
          {items.map((item) => (
            <div key={item.id} draggable onDragStart={() => setDragId(item.id)} onDragOver={(e) => e.preventDefault()} onDrop={() => dropOn(item.id)} className="cursor-grab rounded-lg border border-line bg-void/45 p-4">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div><span className="font-black text-ionHot">{item.phase}</span> <span className="font-black text-white">{item.title}</span></div>
                <span className="rounded-full border border-ionSoft/25 px-3 py-1 text-xs text-ionSoft">{item.status}</span>
              </div>
              <p className="mt-2 text-sm text-frost/62">{item.description}</p>
            </div>
          ))}
        </div>
      </Panel>
    </AdminSection>
  )
}

function FaqModule() {
  const [items, setItems] = useState([])
  const [form, setForm] = useState({ question: '', answer: '', lang: 'en', order: 0 })
  const [error, setError] = useState('')
  const load = () => adminFetch('/api/admin/faq').then((payload) => setItems(payload.items || [])).catch((err) => setError(err.message))
  useEffect(load, [])

  async function create(event) {
    event.preventDefault()
    await adminFetch('/api/admin/faq', {
      method: 'POST',
      body: JSON.stringify({ question: form.question, contentJson: { answer: form.answer }, lang: form.lang, order: Number(form.order) || 0 }),
    })
    setForm({ question: '', answer: '', lang: 'en', order: 0 })
    load()
  }

  async function remove(id) {
    await adminFetch(`/api/admin/faq/${id}`, { method: 'DELETE' })
    load()
  }

  return (
    <AdminSection title="FAQ Management" text="Create, review and remove FAQ entries." error={error}>
      <Panel title="Create FAQ">
        <form onSubmit={create} className="grid gap-3">
          <input className="admin-input" placeholder="Question" value={form.question} onChange={(e) => setForm({ ...form, question: e.target.value })} />
          <textarea className="admin-input min-h-24" placeholder="Answer" value={form.answer} onChange={(e) => setForm({ ...form, answer: e.target.value })} />
          <button className="admin-button w-fit"><Plus size={16} /> Add FAQ</button>
        </form>
      </Panel>
      <Panel title="FAQ items">
        <div className="grid gap-3">
          {items.map((item) => (
            <div key={item.id} className="rounded-lg border border-line bg-void/45 p-4">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <h3 className="font-black text-white">{item.question}</h3>
                  <p className="mt-2 text-sm text-frost/62">{item.contentJson?.answer || stringify(item.contentJson)}</p>
                </div>
                <button className="admin-button secondary" onClick={() => remove(item.id)}>Delete</button>
              </div>
            </div>
          ))}
        </div>
      </Panel>
    </AdminSection>
  )
}

function AuditModule() {
  const [items, setItems] = useState([])
  const [action, setAction] = useState('')
  const [error, setError] = useState('')

  const load = () => {
    const query = action ? `?action=${encodeURIComponent(action)}` : ''
    adminFetch(`/api/admin/audit-log${query}`).then((payload) => setItems(payload.items || [])).catch((err) => setError(err.message))
  }

  useEffect(load, [])

  return (
    <AdminSection title="Audit Log" text="Filter operational changes by action and inspect diffs." error={error}>
      <Panel title="Filters">
        <div className="flex flex-col gap-3 sm:flex-row">
          <input className="admin-input" placeholder="action contains..." value={action} onChange={(e) => setAction(e.target.value)} />
          <button className="admin-button" onClick={load}>Apply</button>
        </div>
      </Panel>
      <Panel title="Entries">
        <div className="grid gap-3">
          {items.map((item) => (
            <details key={item.id} className="rounded-lg border border-line bg-void/45 p-4">
              <summary className="cursor-pointer font-mono text-sm text-ionHot">{item.action} - {item.entity}</summary>
              <pre className="mt-4 overflow-auto rounded-lg bg-black/30 p-4 text-xs text-frost/70">{JSON.stringify(item, null, 2)}</pre>
            </details>
          ))}
        </div>
      </Panel>
    </AdminSection>
  )
}

function Panel({ title, children }) {
  return (
    <section className="rounded-lg border border-line bg-white/[0.025] p-5">
      <h2 className="text-xl font-black text-white">{title}</h2>
      <div className="mt-5">{children}</div>
    </section>
  )
}

function AdminSection({ title, text, error, children }) {
  return (
    <div className="grid gap-5">
      <div>
        <p className="text-sm font-bold uppercase tracking-[0.3em] text-ionSoft/70">Alvenqis Admin</p>
        <h1 className="mt-3 text-4xl font-black text-white">{title}</h1>
        <p className="mt-3 max-w-3xl text-frost/62">{text}</p>
      </div>
      {error && <div className="rounded-lg border border-red-400/30 bg-red-400/10 p-4 text-sm text-red-200">{error}</div>}
      {children}
    </div>
  )
}

function Module({ section }) {
  if (section === 'users') return <UsersModule />
  if (section === 'content') return <ContentModule />
  if (section === 'network') return <NetworkParamsModule />
  if (section === 'roadmap') return <RoadmapModule />
  if (section === 'faq') return <FaqModule />
  if (section === 'audit') return <AuditModule />
  return <Dashboard />
}

export default function AdminApp({ path }) {
  const [user, setUser] = useState(null)
  const [checking, setChecking] = useState(true)
  const section = pathToSection(path)

  useEffect(() => {
    async function check() {
      const existingDevUser = getDevUser()
      if (existingDevUser) {
        setUser(existingDevUser)
        setChecking(false)
        return
      }

      if (!getAccessToken()) {
        const refreshed = await refreshSession()
        setUser(refreshed)
        setChecking(false)
        return
      }

      try {
        const payload = await adminFetch('/auth/me')
        setUser(payload.user)
      } catch {
        setUser(devBypassEnabled ? setDevUser(true) : null)
      } finally {
        setChecking(false)
      }
    }

    check()
  }, [])

  if (checking) {
    return <div className="grid min-h-screen place-items-center bg-void text-frost">Loading admin...</div>
  }

  if (!user) {
    return <LoginScreen onLogin={setUser} />
  }

  return (
    <div className="min-h-screen bg-void text-frost">
      <style>{`
        .admin-input { width: 100%; border-radius: 0.5rem; border: 1px solid rgba(255,255,255,.12); background: rgba(5,6,13,.72); padding: .75rem 1rem; color: #f4f7fb; outline: none; }
        .admin-input:focus { border-color: rgba(77,252,255,.65); }
        .admin-button { display: inline-flex; align-items: center; justify-content: center; gap: .5rem; border-radius: 999px; background: #4dfcff; padding: .75rem 1rem; color: #05060d; font-weight: 900; }
        .admin-button.secondary { border: 1px solid rgba(255,255,255,.12); background: rgba(255,255,255,.04); color: #f4f7fb; }
      `}</style>
      <div className="grid min-h-screen lg:grid-cols-[280px_1fr]">
        <aside className="border-b border-line bg-ink/70 p-5 lg:border-b-0 lg:border-r">
          <Logo />
          <div className="mt-8 grid gap-2">
            {navItems.map(([label, key, Icon]) => (
              <button key={key} onClick={() => navigate(key)} className={`flex items-center gap-3 rounded-lg px-4 py-3 text-left text-sm font-bold transition ${section === key || (section === 'dashboard' && key === 'dashboard') ? 'bg-ionSoft/12 text-ionHot' : 'text-frost/65 hover:bg-white/[0.045] hover:text-white'}`}>
                <Icon size={18} />
                {label}
              </button>
            ))}
          </div>
        </aside>
        <div className="min-w-0">
          <header className="flex flex-col gap-4 border-b border-line bg-void/72 px-5 py-4 backdrop-blur-xl sm:flex-row sm:items-center sm:justify-between">
            <div>
              <div className="text-sm font-black text-white">{user.email}</div>
              <div className="text-xs uppercase tracking-[0.2em] text-ionSoft/70">{user.role}{user.devMode ? ' / dev mode' : ''}</div>
            </div>
            <div className="flex gap-3">
              <a href="/" className="admin-button secondary">Public site</a>
              <button className="admin-button secondary" onClick={async () => { await logout(); setUser(null) }}><LogOut size={16} /> Logout</button>
            </div>
          </header>
          <main className="p-5 lg:p-8">
            <Module section={section} />
          </main>
        </div>
      </div>
    </div>
  )
}
