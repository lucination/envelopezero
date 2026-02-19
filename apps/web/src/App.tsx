import { useEffect, useMemo, useState } from 'react'

type Budget = { id: string; name: string; currency_code: string; is_default: boolean }
type Named = { id: string; name: string; budget_id?: string }
type Category = { id: string; name: string; budget_id: string; supercategory_id: string }
type Split = { category_id: string; inflow: number; outflow: number; memo?: string }
type Transaction = { id: string; budget_id: string; account_id: string; date: string; payee?: string; memo?: string; splits: Split[] }

type Session = { token: string; user_id: string }

const API = '/api'

async function api<T>(path: string, token: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
      ...(init?.headers || {}),
    },
  })
  if (!res.ok) throw new Error(`Request failed: ${res.status}`)
  return (await res.json()) as T
}

export function App() {
  const [session, setSession] = useState<Session | null>(null)
  const [email, setEmail] = useState('')
  const [tokenInput, setTokenInput] = useState('')
  const [notice, setNotice] = useState('')

  const [budgets, setBudgets] = useState<Budget[]>([])
  const [accounts, setAccounts] = useState<Named[]>([])
  const [supercategories, setSupercategories] = useState<Named[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const [transactions, setTransactions] = useState<Transaction[]>([])
  const [dashboard, setDashboard] = useState({ inflow: 0, outflow: 0, available: 0 })

  const activeBudget = useMemo(() => budgets[0], [budgets])

  useEffect(() => {
    const raw = localStorage.getItem('ez_session')
    if (raw) setSession(JSON.parse(raw))

    const url = new URL(window.location.href)
    const token = url.searchParams.get('token')
    if (token) {
      setTokenInput(token)
      url.searchParams.delete('token')
      window.history.replaceState({}, '', url.toString())
    }
  }, [])

  useEffect(() => {
    if (!session) return
    localStorage.setItem('ez_session', JSON.stringify(session))
    refresh(session.token)
  }, [session])

  async function refresh(token = session?.token) {
    if (!token) return
    const [b, a, s, c, t, d] = await Promise.all([
      api<Budget[]>('/budgets', token),
      api<Named[]>('/accounts', token),
      api<Named[]>('/supercategories', token),
      api<Category[]>('/categories', token),
      api<Transaction[]>('/transactions', token),
      api<{ inflow: number; outflow: number; available: number }>('/dashboard', token),
    ])
    setBudgets(b)
    setAccounts(a)
    setSupercategories(s)
    setCategories(c)
    setTransactions(t)
    setDashboard(d)
  }

  async function requestMagicLink(e: React.FormEvent) {
    e.preventDefault()
    const res = await fetch(`${API}/auth/magic-link/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email }),
    })
    const data = await res.json()
    setNotice(`Mail sent. Dev token: ${data.debug_token ?? 'check Mailpit'}`)
  }

  async function verifyToken(e: React.FormEvent) {
    e.preventDefault()
    const res = await fetch(`${API}/auth/magic-link/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: tokenInput }),
    })
    if (!res.ok) {
      setNotice('Token invalid or expired')
      return
    }
    setSession(await res.json())
    setNotice('Signed in')
  }

  if (!session) {
    return (
      <main className="shell">
        <section className="card">
          <h1>EnvelopeZero</h1>
          <form onSubmit={requestMagicLink}>
            <input value={email} onChange={(e) => setEmail(e.target.value)} placeholder="you@example.com" />
            <button type="submit">Send magic link</button>
          </form>
          <form onSubmit={verifyToken}>
            <input value={tokenInput} onChange={(e) => setTokenInput(e.target.value)} placeholder="Paste token" />
            <button type="submit">Verify token</button>
          </form>
          <p className="status">{notice}</p>
        </section>
      </main>
    )
  }

  return (
    <main className="shell wide">
      <section className="card">
        <h2>Dashboard (USD cents)</h2>
        <p>Inflow: {dashboard.inflow} · Outflow: {dashboard.outflow} · Available: {dashboard.available}</p>
      </section>
      <CrudPanel
        title="Budgets"
        onCreate={async (name) => {
          await api('/budgets', session.token, { method: 'POST', body: JSON.stringify({ name, currency_code: 'USD' }) })
          refresh()
        }}
        items={budgets}
      />
      <CrudPanel
        title="Accounts"
        onCreate={async (name) => {
          if (!activeBudget) return
          await api('/accounts', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) })
          refresh()
        }}
        items={accounts}
      />
      <CrudPanel
        title="Supercategories"
        onCreate={async (name) => {
          if (!activeBudget) return
          await api('/supercategories', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) })
          refresh()
        }}
        items={supercategories}
      />
      <CrudPanel
        title="Categories"
        onCreate={async (name) => {
          if (!activeBudget || !supercategories[0]) return
          await api('/categories', session.token, {
            method: 'POST',
            body: JSON.stringify({ name, budget_id: activeBudget.id, supercategory_id: supercategories[0].id }),
          })
          refresh()
        }}
        items={categories}
      />
      <section className="card">
        <h2>Transactions</h2>
        <button
          onClick={async () => {
            if (!activeBudget || !accounts[0] || !categories[0]) return
            await api('/transactions', session.token, {
              method: 'POST',
              body: JSON.stringify({
                budget_id: activeBudget.id,
                account_id: accounts[0].id,
                date: new Date().toISOString().slice(0, 10),
                payee: 'Demo payee',
                memo: 'Sample split',
                splits: [
                  { category_id: categories[0].id, inflow: 0, outflow: 2500, memo: 'Groceries' },
                  { category_id: categories[0].id, inflow: 1000, outflow: 0, memo: 'Refund' },
                ],
              }),
            })
            refresh()
          }}
        >
          Add sample split transaction
        </button>
        <ul>
          {transactions.map((t) => (
            <li key={t.id}>
              {t.date} {t.payee} ({t.splits.length} splits)
            </li>
          ))}
        </ul>
      </section>
    </main>
  )
}

function CrudPanel({ title, items, onCreate }: { title: string; items: any[]; onCreate: (name: string) => Promise<void> }) {
  const [name, setName] = useState('')
  return (
    <section className="card">
      <h2>{title}</h2>
      <form
        onSubmit={async (e) => {
          e.preventDefault()
          if (!name.trim()) return
          await onCreate(name)
          setName('')
        }}
      >
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder={`New ${title}`} />
        <button type="submit">Create</button>
      </form>
      <ul>
        {items.map((x: any) => (
          <li key={x.id}>{x.name}</li>
        ))}
      </ul>
    </section>
  )
}
