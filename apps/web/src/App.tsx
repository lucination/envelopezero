import { useEffect, useMemo, useState } from 'react'

type Budget = { id: string; name: string; currency_code: string; is_default: boolean }
type Named = { id: string; name: string; budget_id?: string }
type Category = { id: string; name: string; budget_id: string; supercategory_id: string }
type Split = { id?: string; category_id: string; inflow: number; outflow: number; memo?: string }
type Transaction = { id: string; budget_id: string; account_id: string; date: string; payee?: string; memo?: string; splits: Split[] }
type Session = { token: string; user_id: string }

const API = '/api'

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

  useEffect(() => {
    const raw = localStorage.getItem('ez_session')
    if (raw) setSession(JSON.parse(raw))
  }, [])

  useEffect(() => {
    if (!session) return
    localStorage.setItem('ez_session', JSON.stringify(session))
    refresh(session.token).catch(() => setNotice('Failed to load data'))
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
    setNotice(`Mail sent. Dev token: ${data.debug_token ?? 'check Mailpit'}`)
    setTokenInput(data.debug_token || '')
  }

  async function verifyToken(e: React.FormEvent) {
    e.preventDefault()
    const res = await fetch(`${API}/auth/magic-link/verify`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ token: tokenInput }) })
    if (!res.ok) return setNotice('Token invalid or expired')
    setSession(await res.json())
  }

  if (!session) return <main className="shell"><section className="card"><h1>EnvelopeZero</h1><form onSubmit={requestMagicLink}><input value={email} onChange={(e) => setEmail(e.target.value)} placeholder="you@example.com" aria-label="Email" /><button>Send magic link</button></form><form onSubmit={verifyToken}><input value={tokenInput} onChange={(e) => setTokenInput(e.target.value)} placeholder="Paste token" aria-label="Token" /><button>Verify token</button></form><p className="status">{notice || 'Sign in to continue'}</p></section></main>

  return (
    <main className="shell wide">
      <section className="card panel-row">
        <h2>Dashboard</h2>
        <p data-testid="dashboard-totals">Inflow: {dashboard.inflow} · Outflow: {dashboard.outflow} · Available: {dashboard.available}</p>
        {loading && <p className="status">Refreshing…</p>}
      </section>

      <section className="card">
        <h2>Category assignment ({activeMonth})</h2>
        <p className="status">Guardrailed capability: disabled unless server flag is on.</p>
        <form onSubmit={async (e) => {
          e.preventDefault(); if (!session || !activeBudget || !categories[0]) return
          await api('/category-assignments', session.token, { method: 'POST', body: JSON.stringify({ budget_id: activeBudget.id, category_id: categories[0].id, month: activeMonth, amount: Number(assignmentAmount) }) })
          setNotice('Assignment saved')
        }}>
          <input aria-label="Assignment amount" type="number" value={assignmentAmount} onChange={(e) => setAssignmentAmount(e.target.value)} />
          <button type="submit">Assign to first category</button>
        </form>
      </section>

      <CrudPanel title="Accounts" items={accounts} parentRequired={!activeBudget} onCreate={async (name) => { if (!session || !activeBudget) return; await api('/accounts', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) }); await refresh() }} />
      <CrudPanel title="Supercategories" items={supercategories} parentRequired={!activeBudget} onCreate={async (name) => { if (!session || !activeBudget) return; await api('/supercategories', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) }); await refresh() }} />
      <CrudPanel title="Categories" items={categories} parentRequired={!activeBudget || !supercategories[0]} onCreate={async (name) => { if (!session || !activeBudget || !supercategories[0]) return; await api('/categories', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id, supercategory_id: supercategories[0].id }) }); await refresh() }} />

      <section className="card">
        <h2>Transactions</h2>
        <form onSubmit={async (e) => { e.preventDefault(); if (!activeBudget || !accounts[0] || !categories[0] || !session) return setNotice('Need budget/account/category'); await api('/transactions', session.token, { method: 'POST', body: JSON.stringify({ budget_id: activeBudget.id, account_id: accounts[0].id, date: txDate, payee: txPayee || null, memo: txMemo || null, splits: [{ category_id: categories[0].id, inflow: Number(txInflow), outflow: Number(txOutflow), memo: null }] }) }); setNotice('Transaction created'); setTxPayee(''); setTxMemo(''); setTxInflow('0'); setTxOutflow('0'); await refresh() }}>
          <input aria-label="Transaction date" type="date" value={txDate} onChange={(e) => setTxDate(e.target.value)} />
          <input aria-label="Payee" value={txPayee} onChange={(e) => setTxPayee(e.target.value)} placeholder="Payee" />
          <input aria-label="Memo" value={txMemo} onChange={(e) => setTxMemo(e.target.value)} placeholder="Memo" />
          <input aria-label="Inflow" type="number" value={txInflow} onChange={(e) => setTxInflow(e.target.value)} />
          <input aria-label="Outflow" type="number" value={txOutflow} onChange={(e) => setTxOutflow(e.target.value)} />
          <button>Create transaction</button>
        </form>
        {!transactions.length && <p className="status">No transactions yet</p>}
      </section>
      <button onClick={() => { localStorage.removeItem('ez_session'); setSession(null) }}>Logout</button>
      <p className="status">{notice}</p>
    </main>
  )
}

function CrudPanel({ title, items, onCreate, parentRequired }: { title: string; items: any[]; onCreate: (name: string) => Promise<void>; parentRequired?: boolean }) {
  const [name, setName] = useState('')
  return <section className="card"><h2>{title}</h2><form onSubmit={async (e) => { e.preventDefault(); if (!name.trim() || parentRequired) return; await onCreate(name); setName('') }}><input aria-label={`New ${title}`} value={name} onChange={(e) => setName(e.target.value)} disabled={parentRequired} /><button disabled={parentRequired}>Create</button></form>{!items.length && <p className="status">No {title.toLowerCase()} yet</p>}<ul>{items.map((x) => <li key={x.id}>{x.name}</li>)}</ul></section>
}
