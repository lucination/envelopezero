import { useEffect, useMemo, useState } from 'react'

type Budget = { id: string; name: string; currency_code: string; is_default: boolean }
type Named = { id: string; name: string; budget_id?: string }
type Category = { id: string; name: string; budget_id: string; supercategory_id: string }
type Split = { id?: string; category_id: string; inflow: number; outflow: number; memo?: string }
type Transaction = { id: string; budget_id: string; account_id: string; date: string; payee?: string; memo?: string; splits: Split[] }
type Session = { token: string; user_id: string }
type AppTab = 'budget' | 'transactions' | 'accounts' | 'settings'
type ToastTone = 'info' | 'success' | 'error'
type Toast = { id: number; message: string; tone: ToastTone }

const API = '/api'
const tabs: { id: AppTab; label: string }[] = [
  { id: 'budget', label: 'Budget' },
  { id: 'transactions', label: 'Transactions' },
  { id: 'accounts', label: 'Accounts' },
  { id: 'settings', label: 'Settings' },
]

async function api<T>(path: string, token: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    ...init,
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}`, ...(init?.headers || {}) },
  })
  if (!res.ok) throw new Error(`Request failed: ${res.status}`)
  if (res.status === 204) return {} as T
  return (await res.json()) as T
}

export function App() {
  const [session, setSession] = useState<Session | null>(null)
  const [email, setEmail] = useState('')
  const [tokenInput, setTokenInput] = useState('')
  const [notice, setNotice] = useState('')
  const [loading, setLoading] = useState(false)
  const [activeTab, setActiveTab] = useState<AppTab>('budget')
  const [toasts, setToasts] = useState<Toast[]>([])

  const [budgets, setBudgets] = useState<Budget[]>([])
  const [accounts, setAccounts] = useState<Named[]>([])
  const [supercategories, setSupercategories] = useState<Named[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const [transactions, setTransactions] = useState<Transaction[]>([])
  const [dashboard, setDashboard] = useState({ inflow: 0, outflow: 0, available: 0 })

  const [txDate, setTxDate] = useState(new Date().toISOString().slice(0, 10))
  const [txPayee, setTxPayee] = useState('')
  const [txMemo, setTxMemo] = useState('')
  const [txInflow, setTxInflow] = useState('0')
  const [txOutflow, setTxOutflow] = useState('0')
  const [assignmentAmount, setAssignmentAmount] = useState('0')

  const activeBudget = useMemo(() => budgets.find((b) => b.is_default) || budgets[0], [budgets])
  const activeMonth = txDate.slice(0, 7)

  function pushToast(message: string, tone: ToastTone = 'info') {
    const id = Date.now() + Math.floor(Math.random() * 1000)
    setToasts((prev) => [...prev, { id, message, tone }])
    setTimeout(() => setToasts((prev) => prev.filter((toast) => toast.id !== id)), tone === 'error' ? 6000 : 3500)
  }

  useEffect(() => {
    const raw = localStorage.getItem('ez_session')
    if (raw) setSession(JSON.parse(raw))
  }, [])

  useEffect(() => {
    if (!session) return
    localStorage.setItem('ez_session', JSON.stringify(session))
    refresh(session.token).catch(() => {
      setNotice('Failed to load data')
      pushToast('Could not load your data. Try again.', 'error')
    })
  }, [session])

  async function refresh(token = session?.token) {
    if (!token) return
    setLoading(true)
    try {
      const [b, a, s, c, t, d] = await Promise.all([
        api<Budget[]>('/budgets', token),
        api<Named[]>('/accounts', token),
        api<Named[]>('/supercategories', token),
        api<Category[]>('/categories', token),
        api<Transaction[]>('/transactions', token),
        api<{ inflow: number; outflow: number; available: number }>('/dashboard', token),
      ])
      setBudgets(b); setAccounts(a); setSupercategories(s); setCategories(c); setTransactions(t); setDashboard(d)
    } finally {
      setLoading(false)
    }
  }

  async function requestMagicLink(e: React.FormEvent) {
    e.preventDefault()
    const res = await fetch(`${API}/auth/magic-link/request`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ email }) })
    const data = await res.json()
    const message = `Mail sent. Dev token: ${data.debug_token ?? 'check Mailpit'}`
    setNotice(message)
    setTokenInput(data.debug_token || '')
    pushToast('Magic link sent. Check your inbox.', 'success')
  }

  async function verifyToken(e: React.FormEvent) {
    e.preventDefault()
    const res = await fetch(`${API}/auth/magic-link/verify`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ token: tokenInput }) })
    if (!res.ok) {
      setNotice('Token invalid or expired')
      pushToast('Token invalid or expired', 'error')
      return
    }
    setSession(await res.json())
    pushToast('Signed in successfully', 'success')
  }

  if (!session) return <main className="auth-shell"><Card><h1>EnvelopeZero</h1><p className="eyebrow">Budget with clarity.</p><form onSubmit={requestMagicLink}><Input value={email} onChange={(e) => setEmail(e.target.value)} placeholder="you@example.com" aria-label="Email" /><Button>Send magic link</Button></form><form onSubmit={verifyToken}><Input value={tokenInput} onChange={(e) => setTokenInput(e.target.value)} placeholder="Paste token" aria-label="Token" /><Button variant="secondary">Verify token</Button></form><p className="status">{notice || 'Sign in to continue'}</p></Card><ToastViewport toasts={toasts} /></main>

  return (
    <div className="app-shell">
      <aside className="sidebar card-like">
        <h2 className="brand">EnvelopeZero</h2>
        <nav className="stack" aria-label="Primary">
          {tabs.map((tab) => (
            <Button key={tab.id} variant={activeTab === tab.id ? 'primary' : 'ghost'} onClick={() => setActiveTab(tab.id)}>
              {tab.label}
            </Button>
          ))}
        </nav>
      </aside>

      <div className="app-main">
        <header className="topbar card-like">
          <div>
            <p className="eyebrow">{activeBudget?.name || 'No budget'}</p>
            <h1>{tabs.find((tab) => tab.id === activeTab)?.label}</h1>
          </div>
          <div className="topbar-actions">
            <p className="status">{loading ? 'Refreshing…' : `Month ${activeMonth}`}</p>
            <Button onClick={() => setActiveTab('transactions')}>Add transaction</Button>
          </div>
        </header>

        <main className="content-grid">
          {activeTab === 'budget' && (
            <>
              <Card className="panel-row">
                <h2>Dashboard</h2>
                <p data-testid="dashboard-totals">Inflow: {dashboard.inflow} · Outflow: {dashboard.outflow} · Available: {dashboard.available}</p>
                <div className="badge-row">
                  <StateBadge tone={dashboard.available < 0 ? 'error' : dashboard.available < 100 ? 'warning' : 'success'}>
                    {dashboard.available < 0 ? 'Overspent' : dashboard.available < 100 ? 'Near zero' : 'On track'}
                  </StateBadge>
                </div>
              </Card>

              <Card>
                <h2>Category assignment ({activeMonth})</h2>
                <p className="status">Guardrailed capability: disabled unless server flag is on.</p>
                <form onSubmit={async (e) => {
                  e.preventDefault(); if (!session || !activeBudget || !categories[0]) return
                  await api('/category-assignments', session.token, { method: 'POST', body: JSON.stringify({ budget_id: activeBudget.id, category_id: categories[0].id, month: activeMonth, amount: Number(assignmentAmount) }) })
                  const message = 'Assignment saved'
                  setNotice(message)
                  pushToast(message, 'success')
                }}>
                  <Input aria-label="Assignment amount" type="number" value={assignmentAmount} onChange={(e) => setAssignmentAmount(e.target.value)} />
                  <Button type="submit">Assign to first category</Button>
                </form>
              </Card>

              <CrudPanel title="Supercategories" items={supercategories} parentRequired={!activeBudget} onCreate={async (name) => { if (!session || !activeBudget) return; await api('/supercategories', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) }); await refresh(); pushToast('Supercategory created', 'success') }} />
              <CrudPanel title="Categories" items={categories} parentRequired={!activeBudget || !supercategories[0]} onCreate={async (name) => { if (!session || !activeBudget || !supercategories[0]) return; await api('/categories', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id, supercategory_id: supercategories[0].id }) }); await refresh(); pushToast('Category created', 'success') }} />
            </>
          )}

          {activeTab === 'transactions' && (
            <Card className="panel-row">
              <h2>Transactions</h2>
              <form onSubmit={async (e) => { e.preventDefault(); if (!activeBudget || !accounts[0] || !categories[0] || !session) { setNotice('Need budget/account/category'); pushToast('Need budget/account/category', 'error'); return } await api('/transactions', session.token, { method: 'POST', body: JSON.stringify({ budget_id: activeBudget.id, account_id: accounts[0].id, date: txDate, payee: txPayee || null, memo: txMemo || null, splits: [{ category_id: categories[0].id, inflow: Number(txInflow), outflow: Number(txOutflow), memo: null }] }) }); setNotice('Transaction created'); pushToast('Transaction created', 'success'); setTxPayee(''); setTxMemo(''); setTxInflow('0'); setTxOutflow('0'); await refresh() }}>
                <Input aria-label="Transaction date" type="date" value={txDate} onChange={(e) => setTxDate(e.target.value)} />
                <Input aria-label="Payee" value={txPayee} onChange={(e) => setTxPayee(e.target.value)} placeholder="Payee" />
                <Input aria-label="Memo" value={txMemo} onChange={(e) => setTxMemo(e.target.value)} placeholder="Memo" />
                <Input aria-label="Inflow" type="number" value={txInflow} onChange={(e) => setTxInflow(e.target.value)} />
                <Input aria-label="Outflow" type="number" value={txOutflow} onChange={(e) => setTxOutflow(e.target.value)} />
                <Button>Create transaction</Button>
              </form>
              {!transactions.length && <p className="status">No transactions yet</p>}
            </Card>
          )}

          {activeTab === 'accounts' && (
            <CrudPanel title="Accounts" items={accounts} parentRequired={!activeBudget} onCreate={async (name) => { if (!session || !activeBudget) return; await api('/accounts', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) }); await refresh(); pushToast('Account created', 'success') }} />
          )}

          {activeTab === 'settings' && (
            <Card className="panel-row">
              <h2>Settings</h2>
              <p className="status">Manage authentication and session controls.</p>
              <Button variant="secondary" onClick={() => { localStorage.removeItem('ez_session'); setSession(null) }}>Logout</Button>
            </Card>
          )}
          <p className="status panel-row">{notice}</p>
        </main>
      </div>

      <nav className="bottom-nav" aria-label="Mobile primary">
        {tabs.map((tab) => (
          <button key={tab.id} className={`tab-btn ${activeTab === tab.id ? 'active' : ''}`} onClick={() => setActiveTab(tab.id)}>{tab.label}</button>
        ))}
      </nav>

      <ToastViewport toasts={toasts} />
    </div>
  )
}

function CrudPanel({ title, items, onCreate, parentRequired }: { title: string; items: any[]; onCreate: (name: string) => Promise<void>; parentRequired?: boolean }) {
  const [name, setName] = useState('')
  return <Card><h2>{title}</h2><form onSubmit={async (e) => { e.preventDefault(); if (!name.trim() || parentRequired) return; await onCreate(name); setName('') }}><Input aria-label={`New ${title}`} value={name} onChange={(e) => setName(e.target.value)} disabled={parentRequired} /><Button disabled={parentRequired}>Create</Button></form>{!items.length && <p className="status">No {title.toLowerCase()} yet</p>}<ul>{items.map((x) => <li key={x.id}>{x.name}</li>)}</ul></Card>
}

function Card({ children, className = '' }: { children: React.ReactNode; className?: string }) {
  return <section className={`card-like ${className}`.trim()}>{children}</section>
}

function Button({ children, variant = 'primary', ...props }: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: 'primary' | 'secondary' | 'ghost' }) {
  return <button className={`btn btn-${variant}`} {...props}>{children}</button>
}

function Input(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return <input className="input" {...props} />
}

function StateBadge({ children, tone = 'success' }: { children: React.ReactNode; tone?: 'success' | 'warning' | 'error' }) {
  return <span className={`badge badge-${tone}`}>{children}</span>
}

function ToastViewport({ toasts }: { toasts: Toast[] }) {
  return <div className="toast-viewport" aria-live="polite">{toasts.map((toast) => <div className={`toast toast-${toast.tone}`} key={toast.id}>{toast.message}</div>)}</div>
}
