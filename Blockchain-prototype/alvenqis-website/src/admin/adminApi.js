const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:4000'
const TOKEN_KEY = 'alvenqis_admin_access_token'
const DEV_USER_KEY = 'alvenqis_admin_dev_user'
const DEV_STATE_KEY = 'alvenqis_admin_dev_state'

export const devBypassEnabled = import.meta.env.DEV && import.meta.env.VITE_ADMIN_DEV_BYPASS !== 'false'

export const devUser = {
  id: 'dev-superadmin',
  email: 'dev@alvenqis.local',
  role: 'superadmin',
  isActive: true,
  lastLogin: new Date().toISOString(),
  devMode: true,
}

const now = () => new Date().toISOString()

function defaultDevState() {
  return {
    users: [
      { id: 'dev-user-1', email: 'dev@alvenqis.local', role: 'superadmin', isActive: true, createdAt: now(), lastLogin: now() },
      { id: 'dev-user-2', email: 'editor@alvenqis.local', role: 'content_editor', isActive: true, createdAt: now(), lastLogin: null },
      { id: 'dev-user-3', email: 'operator@alvenqis.local', role: 'network_operator', isActive: true, createdAt: now(), lastLogin: null },
    ],
    content: [
      { id: 'dev-content-1', pageSlug: 'home', sectionKey: 'hero', lang: 'en', contentJson: { title: 'Alvenqis Core', text: 'Development preview content block.' }, updatedAt: now() },
      { id: 'candidate-content-2', pageSlug: 'status', sectionKey: 'readiness', lang: 'en', contentJson: { mode: 'mainnet_candidate', honest: true }, updatedAt: now() },
    ],
    networkParams: [
      { key: 'block_time_seconds', value: 60, updatedBy: 'dev-user-1', updatedByEmail: 'dev@alvenqis.local', updatedAt: now() },
      { key: 'max_supply', value: '60000000', updatedBy: 'dev-user-1', updatedByEmail: 'dev@alvenqis.local', updatedAt: now() },
      { key: 'halving_interval', value: 1576800, updatedBy: 'dev-user-1', updatedByEmail: 'dev@alvenqis.local', updatedAt: now() },
      { key: 'current_reward', value: '19.02587519', updatedBy: 'dev-user-1', updatedByEmail: 'dev@alvenqis.local', updatedAt: now() },
      { key: 'difficulty_target', value: 'PoW', updatedBy: 'dev-user-1', updatedByEmail: 'dev@alvenqis.local', updatedAt: now() },
    ],
    roadmap: [
      { id: 'dev-roadmap-1', phase: 'Phase 0', title: 'Website and source truth', description: 'Public interface, CMS and honest candidate status.', status: 'active', order: 0 },
      { id: 'dev-roadmap-2', phase: 'Phase 1', title: 'Rust core minimal', description: 'Blocks, transactions, mempool draft, mining and validation.', status: 'next', order: 1 },
      { id: 'candidate-roadmap-3', phase: 'Phase 2', title: 'Mainnet Candidate', description: 'Candidate node, PoW mining, RPC and release hardening.', status: 'in_progress', order: 2 },
    ],
    faq: [
      { id: 'dev-faq-1', question: 'Is this admin connected to a real DB?', contentJson: { answer: 'Not in dev bypass mode. This is local mock data so UI work is never blocked.' }, order: 0, lang: 'en', createdAt: now(), updatedAt: now() },
      { id: 'candidate-faq-2', question: 'Is Alvenqis mainnet live?', contentJson: { answer: 'No. The current network is a Mainnet Candidate and must pass launch gates before public release.' }, order: 1, lang: 'en', createdAt: now(), updatedAt: now() },
    ],
    audit: [
      { id: 'dev-audit-1', userId: 'dev-user-1', userEmail: 'dev@alvenqis.local', action: 'dev.login', entity: 'admin', entityId: 'dev-panel', diffJson: { mode: 'dev_bypass' }, createdAt: now() },
      { id: 'dev-audit-2', userId: 'dev-user-1', userEmail: 'dev@alvenqis.local', action: 'content.previewed', entity: 'content_blocks', entityId: 'dev-content-1', diffJson: { source: 'local_mock' }, createdAt: now() },
    ],
  }
}

function readDevState() {
  const saved = window.localStorage.getItem(DEV_STATE_KEY)
  if (!saved) {
    const state = defaultDevState()
    writeDevState(state)
    return state
  }

  try {
    return JSON.parse(saved)
  } catch {
    const state = defaultDevState()
    writeDevState(state)
    return state
  }
}

function writeDevState(state) {
  window.localStorage.setItem(DEV_STATE_KEY, JSON.stringify(state))
}

function nextId(prefix) {
  if (window.crypto?.randomUUID) return `${prefix}-${window.crypto.randomUUID()}`
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function addAudit(state, action, entity, entityId, diffJson = {}) {
  state.audit.unshift({
    id: nextId('dev-audit'),
    userId: devUser.id,
    userEmail: devUser.email,
    action,
    entity,
    entityId,
    diffJson,
    createdAt: now(),
  })
}

function bodyJson(options) {
  if (!options.body) return {}
  try {
    return JSON.parse(options.body)
  } catch {
    return {}
  }
}

function devResponse(path, options = {}) {
  const method = options.method || 'GET'
  const state = readDevState()
  const url = new URL(path, window.location.origin)
  const pathname = url.pathname

  if (pathname === '/auth/me') return { user: devUser }

  if (pathname === '/api/admin/dashboard') {
    return {
      kpis: {
        candidateHeight: 'offline',
        activeUsers: state.users.filter((user) => user.isActive).length,
        lastLogin: { email: devUser.email, at: devUser.lastLogin },
        contentBlocks: state.content.length,
        roadmapItems: state.roadmap.length,
        faqItems: state.faq.length,
      },
      latestAuditLogs: state.audit.slice(0, 8),
    }
  }

  if (pathname === '/api/admin/users') {
    if (method === 'POST') {
      const body = bodyJson(options)
      const item = { id: nextId('dev-user'), createdAt: now(), lastLogin: null, isActive: true, ...body }
      state.users.unshift(item)
      addAudit(state, 'user.created', 'users', item.id, { after: item })
      writeDevState(state)
      return { item }
    }
    return { items: state.users }
  }

  if (pathname.startsWith('/api/admin/users/') && method === 'PUT') {
    const id = pathname.split('/').pop()
    const body = bodyJson(options)
    const index = state.users.findIndex((item) => item.id === id)
    if (index >= 0) {
      const before = state.users[index]
      state.users[index] = { ...before, ...body }
      addAudit(state, 'user.updated', 'users', id, { before, after: state.users[index] })
      writeDevState(state)
      return { item: state.users[index] }
    }
    return { item: null }
  }

  if (pathname === '/api/admin/content') {
    if (method === 'POST') {
      const body = bodyJson(options)
      const item = { id: nextId('dev-content'), updatedAt: now(), ...body }
      state.content.unshift(item)
      addAudit(state, 'content_block.created', 'content_blocks', item.id, { after: item })
      writeDevState(state)
      return { item }
    }
    return { items: state.content }
  }

  if (pathname.startsWith('/api/admin/content/') && method === 'PUT') {
    const id = pathname.split('/').pop()
    const body = bodyJson(options)
    const index = state.content.findIndex((item) => item.id === id)
    if (index >= 0) {
      const before = state.content[index]
      state.content[index] = { ...before, ...body, updatedAt: now() }
      addAudit(state, 'content_block.updated', 'content_blocks', id, { before, after: state.content[index] })
      writeDevState(state)
      return { item: state.content[index] }
    }
    return { item: null }
  }

  if (pathname === '/api/admin/network-params') {
    return { items: state.networkParams }
  }

  if (pathname.startsWith('/api/admin/network-params/') && method === 'PUT') {
    const key = pathname.split('/').pop()
    const body = bodyJson(options)
    const index = state.networkParams.findIndex((item) => item.key === key)
    const item = { key, value: body.value, updatedBy: devUser.id, updatedByEmail: devUser.email, updatedAt: now() }
    if (index >= 0) state.networkParams[index] = { ...state.networkParams[index], ...item }
    else state.networkParams.push(item)
    addAudit(state, 'network_param.updated', 'network_params', key, { after: item })
    writeDevState(state)
    return { item }
  }

  if (pathname === '/api/admin/roadmap') {
    if (method === 'POST') {
      const body = bodyJson(options)
      const item = { id: nextId('dev-roadmap'), order: state.roadmap.length, ...body }
      state.roadmap.push(item)
      addAudit(state, 'roadmap_item.created', 'roadmap_items', item.id, { after: item })
      writeDevState(state)
      return { item }
    }
    return { items: [...state.roadmap].sort((a, b) => a.order - b.order) }
  }

  if (pathname.startsWith('/api/admin/roadmap/') && method === 'PUT') {
    const id = pathname.split('/').pop()
    const body = bodyJson(options)
    const index = state.roadmap.findIndex((item) => item.id === id)
    if (index >= 0) {
      const before = state.roadmap[index]
      state.roadmap[index] = { ...before, ...body }
      addAudit(state, 'roadmap_item.updated', 'roadmap_items', id, { before, after: state.roadmap[index] })
      writeDevState(state)
      return { item: state.roadmap[index] }
    }
    return { item: null }
  }

  if (pathname === '/api/admin/faq') {
    if (method === 'POST') {
      const body = bodyJson(options)
      const item = { id: nextId('dev-faq'), createdAt: now(), updatedAt: now(), ...body }
      state.faq.push(item)
      addAudit(state, 'faq_item.created', 'faq_items', item.id, { after: item })
      writeDevState(state)
      return { item }
    }
    return { items: [...state.faq].sort((a, b) => a.order - b.order) }
  }

  if (pathname.startsWith('/api/admin/faq/') && method === 'DELETE') {
    const id = pathname.split('/').pop()
    const before = state.faq.find((item) => item.id === id)
    state.faq = state.faq.filter((item) => item.id !== id)
    addAudit(state, 'faq_item.deleted', 'faq_items', id, { before })
    writeDevState(state)
    return null
  }

  if (pathname === '/api/admin/audit-log') {
    const action = url.searchParams.get('action')
    const items = action ? state.audit.filter((item) => item.action.includes(action)) : state.audit
    return { items, total: items.length, limit: items.length, offset: 0 }
  }

  return { items: [] }
}

export function getAccessToken() {
  return window.localStorage.getItem(TOKEN_KEY)
}

export function getDevUser() {
  if (!devBypassEnabled) return null
  return window.localStorage.getItem(DEV_USER_KEY) ? devUser : null
}

export function setDevUser(enabled) {
  if (enabled && devBypassEnabled) {
    window.localStorage.setItem(DEV_USER_KEY, '1')
    setAccessToken('dev-access-token')
    return devUser
  }

  window.localStorage.removeItem(DEV_USER_KEY)
  if (getAccessToken() === 'dev-access-token') {
    setAccessToken(null)
  }
  return null
}

export function setAccessToken(token) {
  if (token) {
    window.localStorage.setItem(TOKEN_KEY, token)
    return
  }

  window.localStorage.removeItem(TOKEN_KEY)
}

export async function adminFetch(path, options = {}) {
  if (getDevUser()) {
    return devResponse(path, options)
  }

  const token = getAccessToken()
  const headers = {
    ...(options.body ? { 'Content-Type': 'application/json' } : {}),
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...options.headers,
  }

  let response = await fetch(`${API_BASE_URL}${path}`, {
    credentials: 'include',
    ...options,
    headers,
  })

  if (response.status === 401 && path !== '/auth/refresh') {
    const refreshed = await refreshSession()
    if (refreshed) {
      response = await fetch(`${API_BASE_URL}${path}`, {
        credentials: 'include',
        ...options,
        headers: {
          ...headers,
          Authorization: `Bearer ${getAccessToken()}`,
        },
      })
    }
  }

  if (response.status === 204) return null

  const payload = await response.json().catch(() => ({}))
  if (!response.ok) {
    throw new Error(payload.error || `Request failed with ${response.status}`)
  }

  return payload
}

export async function login(email, password) {
  if (devBypassEnabled && email === 'dev@alvenqis.local') {
    return setDevUser(true)
  }

  const payload = await adminFetch('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email, password }),
  })
  setAccessToken(payload.accessToken)
  return payload.user
}

export async function refreshSession() {
  const existingDevUser = getDevUser()
  if (existingDevUser) return existingDevUser

  try {
    const payload = await fetch(`${API_BASE_URL}/auth/refresh`, {
      method: 'POST',
      credentials: 'include',
    }).then(async (response) => {
      const body = await response.json().catch(() => ({}))
      if (!response.ok) throw new Error(body.error || 'Refresh failed')
      return body
    })

    setAccessToken(payload.accessToken)
    return payload.user
  } catch {
    setAccessToken(null)
    return null
  }
}

export async function logout() {
  if (!getDevUser()) {
    await fetch(`${API_BASE_URL}/auth/logout`, {
      method: 'POST',
      credentials: 'include',
    }).catch(() => null)
  }
  setDevUser(false)
  setAccessToken(null)
}
