import { useEffect, useMemo, useState } from 'react'

type Budget = { id: string; name: string; currency_code: string; is_default: boolean }
type Named = { id: string; name: string; budget_id?: string }
type Category = { id: string; name: string; budget_id: string; supercategory_id: string }
type Split = { id?: string; category_id: string; inflow: number; outflow: number; memo?: string }
type Transaction = {
  id: string
  budget_id: string
  account_id: string
  date: string
  payee?: string
  memo?: string
  splits: Split[]
}

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
  if (res.status === 204) return {} as T
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

  const [txDate, setTxDate] = useState(new Date().toISOString().slice(0, 10))
  const [txPayee, setTxPayee] = useState('')
  const [txMemo, setTxMemo] = useState('')
  const [txInflow, setTxInflow] = useState('0')
  const [txOutflow, setTxOutflow] = useState('0')
  const [txEditId, setTxEditId] = useState<string | null>(null)

  const activeBudget = useMemo(() => budgets.find((b) => b.is_default) || budgets[0], [budgets])

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
    if (!tokenInput || session) return
    verifyTokenValue(tokenInput).catch(() => {
      setNotice('Token invalid or expired')
    })
  }, [tokenInput, session])

  useEffect(() => {
    if (!session) return
    localStorage.setItem('ez_session', JSON.stringify(session))
    refresh(session.token).catch(() => setNotice('Failed to load data'))
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

  async function verifyTokenValue(token: string) {
    const res = await fetch(`${API}/auth/magic-link/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token }),
    })
    if (!res.ok) throw new Error('invalid token')
    setSession(await res.json())
    setNotice('Signed in')
  }

  async function verifyToken(e: React.FormEvent) {
    e.preventDefault()
    try {
      await verifyTokenValue(tokenInput)
    } catch {
      setNotice('Token invalid or expired')
    }
  }

  function logout() {
    localStorage.removeItem('ez_session')
    setSession(null)
    setBudgets([])
    setAccounts([])
    setSupercategories([])
    setCategories([])
    setTransactions([])
    setNotice('Signed out')
  }

  async function saveTransaction(e: React.FormEvent) {
    e.preventDefault()
    if (!activeBudget || !accounts[0] || !categories[0] || !session) {
      setNotice('Need budget, account, and category before creating transaction')
      return
    }

    const payload = {
      budget_id: activeBudget.id,
      account_id: accounts[0].id,
      date: txDate,
      payee: txPayee || null,
      memo: txMemo || null,
      splits: [
        {
          category_id: categories[0].id,
          inflow: Number(txInflow || '0'),
          outflow: Number(txOutflow || '0'),
          memo: 'Primary split',
        },
      ],
    }

    if (txEditId) {
      await api(`/transactions/${txEditId}`, session.token, { method: 'PUT', body: JSON.stringify(payload) })
      setNotice('Transaction updated')
      setTxEditId(null)
    } else {
      await api('/transactions', session.token, { method: 'POST', body: JSON.stringify(payload) })
      setNotice('Transaction created')
    }

    setTxPayee('')
    setTxMemo('')
    setTxInflow('0')
    setTxOutflow('0')
    await refresh()
  }

  if (!session) {
    return (
      <main className="shell">
        <section className="card">
          <h1>EnvelopeZero</h1>
          <form onSubmit={requestMagicLink}>
            <input value={email} onChange={(e) => setEmail(e.target.value)} placeholder="you@example.com" aria-label="Email" />
            <button type="submit">Send magic link</button>
          </form>
          <form onSubmit={verifyToken}>
            <input value={tokenInput} onChange={(e) => setTokenInput(e.target.value)} placeholder="Paste token" aria-label="Token" />
            <button type="submit">Verify token</button>
          </form>
          <p className="status">{notice || 'Sign in to continue'}</p>
        </section>
      </main>
    )
  }

  return (
    <main className="shell wide">
      <section className="card">
        <h2>Dashboard (USD cents)</h2>
        <p>Inflow: {dashboard.inflow} · Outflow: {dashboard.outflow} · Available: {dashboard.available}</p>
        <button onClick={logout}>Logout</button>
      </section>

      <CrudPanel
        title="Budgets"
        parentRequired={false}
        onCreate={async (name) => {
          if (!session) return
          try {
            await api('/budgets', session.token, { method: 'POST', body: JSON.stringify({ name, currency_code: 'USD' }) })
            setNotice('Budget created')
          } catch {
            setNotice('Multi-budget feature is disabled for MVP')
          }
          await refresh()
        }}
        onUpdate={undefined}
        onDelete={undefined}
        items={budgets}
      />

      <CrudPanel
        title="Accounts"
        parentRequired={!activeBudget}
        onCreate={async (name) => {
          if (!activeBudget || !session) return
          await api('/accounts', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) })
          await refresh()
        }}
        onUpdate={async (id, name) => {
          if (!activeBudget || !session) return
          await api(`/accounts/${id}`, session.token, { method: 'PUT', body: JSON.stringify({ name, budget_id: activeBudget.id }) })
          await refresh()
        }}
        onDelete={async (id) => {
          if (!session) return
          await api(`/accounts/${id}`, session.token, { method: 'DELETE' })
          await refresh()
        }}
        items={accounts}
      />

      <CrudPanel
        title="Supercategories"
        parentRequired={!activeBudget}
        onCreate={async (name) => {
          if (!activeBudget || !session) return
          await api('/supercategories', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) })
          await refresh()
        }}
        onUpdate={async (id, name) => {
          if (!activeBudget || !session) return
          await api(`/supercategories/${id}`, session.token, {
            method: 'PUT',
            body: JSON.stringify({ name, budget_id: activeBudget.id }),
          })
          await refresh()
        }}
        onDelete={async (id) => {
          if (!session) return
          await api(`/supercategories/${id}`, session.token, { method: 'DELETE' })
          await refresh()
        }}
        items={supercategories}
      />

      <CrudPanel
        title="Categories"
        parentRequired={!activeBudget || !supercategories[0]}
        parentHint={!supercategories[0] ? 'Create a supercategory first' : undefined}
        onCreate={async (name) => {
          if (!activeBudget || !supercategories[0] || !session) return
          await api('/categories', session.token, {
            method: 'POST',
            body: JSON.stringify({ name, budget_id: activeBudget.id, supercategory_id: supercategories[0].id }),
          })
          await refresh()
        }}
        onUpdate={async (id, name) => {
          if (!activeBudget || !supercategories[0] || !session) return
          await api(`/categories/${id}`, session.token, {
            method: 'PUT',
            body: JSON.stringify({ name, budget_id: activeBudget.id, supercategory_id: supercategories[0].id }),
          })
          await refresh()
        }}
        onDelete={async (id) => {
          if (!session) return
          await api(`/categories/${id}`, session.token, { method: 'DELETE' })
          await refresh()
        }}
        items={categories}
      />

      <section className="card">
        <h2>Transactions</h2>
        <form onSubmit={saveTransaction}>
          <input aria-label="Transaction date" type="date" value={txDate} onChange={(e) => setTxDate(e.target.value)} />
          <input aria-label="Payee" value={txPayee} onChange={(e) => setTxPayee(e.target.value)} placeholder="Payee" />
          <input aria-label="Memo" value={txMemo} onChange={(e) => setTxMemo(e.target.value)} placeholder="Memo" />
          <input aria-label="Inflow" type="number" value={txInflow} onChange={(e) => setTxInflow(e.target.value)} placeholder="Inflow" />
          <input aria-label="Outflow" type="number" value={txOutflow} onChange={(e) => setTxOutflow(e.target.value)} placeholder="Outflow" />
          <button type="submit">{txEditId ? 'Update transaction' : 'Create transaction'}</button>
          {txEditId && <button onClick={() => setTxEditId(null)}>Cancel edit</button>}
        </form>
        {!transactions.length && <p className="status">No transactions yet</p>}
        <ul>
          {transactions.map((t) => (
            <li key={t.id}>
              {t.date} {t.payee || '—'} ({t.splits.length} splits)
              <button
                onClick={() => {
                  const split = t.splits[0]
                  setTxEditId(t.id)
                  setTxDate(t.date)
                  setTxPayee(t.payee || '')
                  setTxMemo(t.memo || '')
                  setTxInflow(String(split?.inflow || 0))
                  setTxOutflow(String(split?.outflow || 0))
                }}
              >
                Edit
              </button>
              <button
                onClick={async () => {
                  if (!session) return
                  await api(`/transactions/${t.id}`, session.token, { method: 'DELETE' })
                  await refresh()
                }}
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      </section>
      <p className="status">{notice}</p>
    </main>
  )
}

function CrudPanel({
  title,
  items,
  onCreate,
  onUpdate,
  onDelete,
  parentRequired,
  parentHint,
}: {
  title: string
  items: any[]
  onCreate: (name: string) => Promise<void>
  onUpdate?: (id: string, name: string) => Promise<void>
  onDelete?: (id: string) => Promise<void>
  parentRequired?: boolean
  parentHint?: string
}) {
  const [name, setName] = useState('')
  const [edit, setEdit] = useState<string | null>(null)

  return (
    <section className="card">
      <h2>{title}</h2>
      {parentRequired && <p className="status">Blocked: required dependency missing. {parentHint || ''}</p>}
      <form
        onSubmit={async (e) => {
          e.preventDefault()
          if (!name.trim() || parentRequired) return
          if (edit && onUpdate) {
            await onUpdate(edit, name)
            setEdit(null)
          } else {
            await onCreate(name)
          }
          setName('')
        }}
      >
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder={`New ${title}`} aria-label={`New ${title}`} disabled={parentRequired} />
        <button type="submit" disabled={parentRequired}>{edit ? 'Save' : 'Create'}</button>
      </form>
      {!items.length && <p className="status">No {title.toLowerCase()} yet</p>}
      <ul>
        {items.map((x: any) => (
          <li key={x.id}>
            {x.name}
            {onUpdate && (
              <button
                onClick={() => {
                  setEdit(x.id)
                  setName(x.name)
                }}
              >
                Edit
              </button>
            )}
            {onDelete && <button onClick={() => onDelete(x.id)}>Delete</button>}
          </li>
        ))}
      </ul>
    </section>
  )
}
