import { Fragment, useEffect, useMemo, useState, type FormEvent, type ReactNode } from 'react'
import { ArrowLeft, ArrowRight, CircleUserRound, Landmark, PiggyBank, Plus, ReceiptText, Settings, WalletCards } from 'lucide-react'
import { Badge } from './components/ui/badge'
import { Button } from './components/ui/button'
import { Input } from './components/ui/input'
import { cn } from './lib/utils'

type Budget = { id: string; name: string; currency_code: string; is_default: boolean }
type Named = { id: string; name: string; budget_id?: string }
type Category = { id: string; name: string; budget_id: string; supercategory_id: string }
type Split = { id?: string; category_id: string; inflow: number; outflow: number; memo?: string }
type Transaction = { id: string; budget_id: string; account_id: string; date: string; payee?: string; memo?: string; splits: Split[] }
type Session = { token: string; user_id: string }
type AppTab = 'budget' | 'transactions' | 'accounts' | 'settings'
type ToastTone = 'info' | 'success' | 'error'
type Toast = { id: number; message: string; tone: ToastTone }
type CategoryProjection = { category_id: string; assigned: number; activity: number; available: number }
type BudgetRow = { categoryId: string; categoryName: string; supercategoryId: string; supercategoryName: string; assigned: number; activity: number; available: number }

class ApiError extends Error { constructor(public status: number, message: string) { super(message) }}

const API = '/api'
const tabs: { id: AppTab; label: string; icon: ReactNode }[] = [
  { id: 'budget', label: 'Budget', icon: <PiggyBank className="h-4 w-4" /> },
  { id: 'transactions', label: 'Transactions', icon: <ReceiptText className="h-4 w-4" /> },
  { id: 'accounts', label: 'Accounts', icon: <Landmark className="h-4 w-4" /> },
  { id: 'settings', label: 'Settings', icon: <Settings className="h-4 w-4" /> },
]

const mobileNav = [
  { id: 'budget' as AppTab, label: 'Budget', icon: PiggyBank },
  { id: 'transactions' as AppTab, label: 'Moves', icon: WalletCards },
  { id: 'accounts' as AppTab, label: 'Accounts', icon: Landmark },
  { id: 'settings' as AppTab, label: 'Settings', icon: Settings },
  { id: 'settings' as AppTab, label: 'Profile', icon: CircleUserRound },
]

async function api<T>(path: string, token: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API}${path}`, { ...init, headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}`, ...(init?.headers || {}) } })
  if (!res.ok) throw new ApiError(res.status, `Request failed: ${res.status}`)
  if (res.status === 204) return {} as T
  return (await res.json()) as T
}

function currency(amount: number) { return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }).format(amount / 100) }
function monthShift(month: string, direction: -1 | 1) { const [y, m] = month.split('-').map(Number); const d = new Date(y, m - 1 + direction, 1); return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}` }

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
  const [projections, setProjections] = useState<CategoryProjection[]>([])
  const [month, setMonth] = useState(new Date().toISOString().slice(0, 7))
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null)
  const [editingCategoryId, setEditingCategoryId] = useState<string | null>(null)
  const [editingAmount, setEditingAmount] = useState('0')
  const [assignmentsEnabled, setAssignmentsEnabled] = useState(true)
  const [txDate, setTxDate] = useState(new Date().toISOString().slice(0, 10))
  const [txPayee, setTxPayee] = useState('')
  const [txMemo, setTxMemo] = useState('')
  const [txInflow, setTxInflow] = useState('0')
  const [txOutflow, setTxOutflow] = useState('0')

  const activeBudget = useMemo(() => budgets.find((b) => b.is_default) || budgets[0], [budgets])
  const pushToast = (message: string, tone: ToastTone = 'info') => { const id = Date.now(); setToasts((p) => [...p, { id, message, tone }]); setTimeout(() => setToasts((p) => p.filter((t) => t.id !== id)), 3200) }

  useEffect(() => { const raw = localStorage.getItem('ez_session'); if (raw) setSession(JSON.parse(raw)) }, [])
  useEffect(() => { if (!session) return; localStorage.setItem('ez_session', JSON.stringify(session)); refresh(session.token).catch(() => pushToast('Could not load your data.', 'error')) }, [session])
  useEffect(() => { if (!session) return; refreshMonthProjection(session.token).catch(() => pushToast('Could not refresh budget projection.', 'error')) }, [month, session, categories.length])

  async function refresh(token = session?.token) {
    if (!token) return
    setLoading(true)
    try {
      const [b, a, s, c, t, d] = await Promise.all([
        api<Budget[]>('/budgets', token), api<Named[]>('/accounts', token), api<Named[]>('/supercategories', token), api<Category[]>('/categories', token), api<Transaction[]>('/transactions', token), api<{ inflow: number; outflow: number; available: number }>('/dashboard', token),
      ])
      setBudgets(b); setAccounts(a); setSupercategories(s); setCategories(c); setTransactions(t); setDashboard(d)
    } finally { setLoading(false) }
  }

  async function refreshMonthProjection(token = session?.token) { if (!token) return; setProjections(await api<CategoryProjection[]>(`/projections/month/${month}`, token)) }
  async function requestMagicLink(e: FormEvent) { e.preventDefault(); const res = await fetch(`${API}/auth/magic-link/request`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ email }) }); const data = await res.json(); setNotice(`Mail sent. Dev token: ${data.debug_token ?? 'check Mailpit'}`); setTokenInput(data.debug_token || '') }
  async function verifyToken(e: FormEvent) { e.preventDefault(); const res = await fetch(`${API}/auth/magic-link/verify`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ token: tokenInput }) }); if (!res.ok) return setNotice('Token invalid or expired'); setSession(await res.json()) }

  const budgetRows = useMemo<BudgetRow[]>(() => {
    const pmap = new Map(projections.map((p) => [p.category_id, p])); const smap = new Map(supercategories.map((s) => [s.id, s.name]))
    return categories.filter((c) => !activeBudget || c.budget_id === activeBudget.id).map((c) => ({ categoryId: c.id, categoryName: c.name, supercategoryId: c.supercategory_id, supercategoryName: smap.get(c.supercategory_id) || 'Uncategorized', assigned: pmap.get(c.id)?.assigned ?? 0, activity: pmap.get(c.id)?.activity ?? 0, available: pmap.get(c.id)?.available ?? 0 }))
  }, [activeBudget, categories, projections, supercategories])
  const groupedRows = useMemo(() => { const grouped = new Map<string, { name: string; rows: BudgetRow[] }>(); budgetRows.forEach((row) => { if (!grouped.has(row.supercategoryId)) grouped.set(row.supercategoryId, { name: row.supercategoryName, rows: [] }); grouped.get(row.supercategoryId)?.rows.push(row) }); return [...grouped.entries()].map(([id, group]) => ({ id, ...group })) }, [budgetRows])
  const readyToAssign = useMemo(() => dashboard.available - budgetRows.reduce((sum, row) => sum + row.assigned, 0), [dashboard.available, budgetRows])
  const overspentRows = useMemo(() => budgetRows.filter((row) => row.available < 0), [budgetRows])
  const selectedRow = useMemo(() => budgetRows.find((r) => r.categoryId === selectedCategoryId) || budgetRows[0], [selectedCategoryId, budgetRows])

  async function saveAssignment(categoryId: string, nextAssignedAbsolute: number) {
    if (!session || !activeBudget) return
    const current = budgetRows.find((row) => row.categoryId === categoryId); if (!current) return
    const delta = nextAssignedAbsolute - current.assigned; if (delta === 0) return setEditingCategoryId(null)
    try {
      await api('/category-assignments', session.token, { method: 'POST', body: JSON.stringify({ budget_id: activeBudget.id, category_id: categoryId, month, amount: delta }) })
      setEditingCategoryId(null); await refreshMonthProjection(session.token); await refresh(session.token)
    } catch (error) {
      if (error instanceof ApiError && (error.status === 404 || error.status === 501)) { setAssignmentsEnabled(false); setEditingCategoryId(null); return }
      pushToast('Could not save assignment.', 'error')
    }
  }

  if (!session) return <main className="min-h-screen p-4 sm:grid sm:place-items-center"><section className="mx-auto w-full max-w-md p-5"><header className="mb-4 border-b border-white/10 pb-3"><h1 className="font-brand text-3xl">EnvelopeZero</h1><p className="text-sm text-muted-foreground">Professional zero-based budgeting.</p></header><div className="space-y-3"><form className="space-y-2" onSubmit={requestMagicLink}><Input value={email} onChange={(e) => setEmail(e.target.value)} placeholder="you@example.com" aria-label="Email" /><Button className="w-full">Send magic link</Button></form><form className="space-y-2" onSubmit={verifyToken}><Input value={tokenInput} onChange={(e) => setTokenInput(e.target.value)} placeholder="Paste token" aria-label="Token" /><Button variant="secondary" className="w-full">Verify token</Button></form><p className="text-xs text-muted-foreground">{notice || 'Sign in to continue'}</p></div></section><ToastViewport toasts={toasts} /></main>

  return <div className="min-h-screen md:grid md:grid-cols-[188px_1fr_232px] md:gap-0">
    <aside className="workspace-pane ez-panel hidden border-r border-white/10 p-4 md:block">
      <h2 className="font-brand text-[1.7rem] font-bold leading-none">EnvelopeZero</h2>
      <p className="mb-3 mt-0.5 text-[10px] uppercase tracking-[0.16em] text-muted-foreground">Budget Workspace</p>
      <nav className="space-y-0.5" aria-label="Primary">
        {tabs.map((tab) => <Button key={tab.id} variant={activeTab === tab.id ? 'default' : 'ghost'} className="w-full justify-start" onClick={() => setActiveTab(tab.id)}>{tab.icon}{tab.label}</Button>)}
      </nav>
    </aside>

    <main className="workspace-pane p-2 pb-32 md:px-4 md:pb-0">
      <header className="workspace-strip mb-2 hidden items-center justify-between border-b border-white/10 px-1 pb-3 pt-4 md:flex">
        <div>
          <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">{activeBudget?.name || 'No budget'}</p>
          <h1 className="font-brand text-3xl">{tabs.find((t) => t.id === activeTab)?.label}</h1>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Badge variant="outline">{loading ? 'Refreshing…' : 'Synced'}</Badge>
          <Badge variant="outline">Month {month}</Badge>
          <Button size="sm" onClick={() => setActiveTab('transactions')}>+ Transaction</Button>
        </div>
      </header>

      <header className="workspace-strip mb-2 md:hidden">
        <div className="flex items-center justify-between border-b border-white/10 px-1 pb-2">
          <button className="ios-icon-btn" onClick={() => setMonth((m) => monthShift(m, -1))} aria-label="Previous month"><ArrowLeft className="h-4 w-4" /></button>
          <p className="text-sm font-semibold tracking-wide">{month}</p>
          <button className="ios-icon-btn" onClick={() => setMonth((m) => monthShift(m, 1))} aria-label="Next month"><ArrowRight className="h-4 w-4" /></button>
        </div>
      </header>

      {activeTab === 'budget' && <section className="workspace-strip space-y-2" data-testid="budget-workspace">
        <div data-testid="dashboard-totals" className="grid gap-2 border-b border-white/10 pb-2 md:grid-cols-[1fr_auto]">
          <div><p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Ready to Assign</p><p data-testid="ready-to-assign" className={cn('text-3xl font-extrabold', readyToAssign < 0 ? 'text-danger' : 'text-success')}>{currency(readyToAssign)}</p></div>
          <div className="hidden items-center gap-2 md:flex"><Button variant="outline" size="sm" onClick={() => setMonth((m) => monthShift(m, -1))}><ArrowLeft className="h-4 w-4" />Previous</Button><span className="w-20 text-center text-sm font-bold">{month}</span><Button variant="outline" size="sm" onClick={() => setMonth((m) => monthShift(m, 1))}>Next<ArrowRight className="h-4 w-4" /></Button></div>
          <div className="flex gap-2 md:col-span-2"><Badge variant="outline">{budgetRows.length} categories</Badge><Badge variant="outline">{overspentRows.length} overspent</Badge></div>
        </div>

        {overspentRows.length > 0 && <div className="flex items-center justify-between border-l-2 border-rose-400/60 bg-rose-500/10 px-2 py-1.5"><div><p className="text-[10px] uppercase tracking-[0.14em] text-rose-300">Overspent</p><p className="text-xs text-rose-100">{overspentRows[0].categoryName} needs coverage.</p></div><button className="bg-rose-500/25 px-2 py-0.5 text-[11px] font-semibold text-rose-100">Cover</button></div>}

        <div className="hidden overflow-x-auto md:block"><table className="ez-table min-w-full text-[13px]"><thead className="border-b border-white/10 text-muted-foreground"><tr><th className="px-3 py-1.5 text-left text-[10px] font-bold uppercase tracking-[0.14em]">Category</th><th className="px-3 py-1.5 text-right text-[10px] font-bold uppercase tracking-[0.14em]">Assigned</th><th className="px-3 py-1.5 text-right text-[10px] font-bold uppercase tracking-[0.14em]">Activity</th><th className="px-3 py-1.5 text-right text-[10px] font-bold uppercase tracking-[0.14em]">Available</th></tr></thead><tbody>
          {groupedRows.map((group) => <Fragment key={group.id}>
            <tr><td colSpan={4} className="px-3 py-1 text-[10px] font-bold uppercase tracking-[0.14em] text-muted-foreground">{group.name}</td></tr>
            {group.rows.map((row) => { const isSelected = selectedCategoryId === row.categoryId; const isEditing = editingCategoryId === row.categoryId; return <tr key={row.categoryId} data-testid={`budget-row-${row.categoryId}`} onClick={() => setSelectedCategoryId(row.categoryId)} className={cn('ez-row border-t border-white/5 cursor-pointer', isSelected && 'bg-muted/50')}><td className="px-3 py-1.5"><div className="flex items-center gap-1.5"><span className="font-semibold">{row.categoryName}</span>{row.available < 0 && <Badge variant="danger">Overspent</Badge>}</div></td><td className="px-3 py-1.5 text-right">{isEditing ? <div className="ml-auto flex max-w-[240px] items-center gap-2"><Input aria-label={`Assigned amount for ${row.categoryName}`} type="number" value={editingAmount} onChange={(e) => setEditingAmount(e.target.value)} /><Button size="sm" onClick={(e) => { e.stopPropagation(); saveAssignment(row.categoryId, Number(editingAmount)) }} disabled={!assignmentsEnabled}>Save</Button></div> : <button className="px-2 py-1 font-bold text-primary transition hover:bg-primary/10 disabled:text-muted-foreground" disabled={!assignmentsEnabled} onClick={(e) => { e.stopPropagation(); setEditingCategoryId(row.categoryId); setEditingAmount(String(row.assigned)) }}>{currency(row.assigned)}</button>}</td><td className="px-3 py-1.5 text-right text-muted-foreground">{currency(row.activity)}</td><td className="px-3 py-1.5 text-right"><AvailabilityChip amount={row.available} /></td></tr> })}
          </Fragment>)}
          {!groupedRows.length && <tr><td colSpan={4} className="px-4 py-8 text-center text-muted-foreground">No categories yet. Add categories to start assigning money.</td></tr>}
        </tbody></table></div>

        <div className="space-y-1.5 md:hidden">
          {groupedRows.map((group) => <section key={group.id} className="border-b border-white/10 pb-1">
            <p className="px-1 pb-0.5 text-[10px] font-bold uppercase tracking-[0.14em] text-muted-foreground">{group.name}</p>
            {group.rows.map((row) => <button key={row.categoryId} onClick={() => setSelectedCategoryId(row.categoryId)} className="flex w-full items-center gap-2 border-t border-white/10 px-1 py-1.5 text-left first:border-t-0"><span className="text-sm">{row.available < 0 ? '⚠️' : '💼'}</span><span className="min-w-0 flex-1"><span className="block truncate text-sm font-semibold">{row.categoryName}</span><span className="block text-[10px] text-muted-foreground">Assigned {currency(row.assigned)} • Activity {currency(row.activity)}</span></span><AvailabilityChip amount={row.available} /></button>)}
          </section>)}
        </div>
      </section>}

      {activeTab === 'transactions' && <section className="space-y-4 border-t border-white/10 pt-3"><header><h2 className="font-brand text-2xl">Transactions</h2></header><form className="grid gap-2 md:grid-cols-2" onSubmit={async (e) => { e.preventDefault(); if (!activeBudget || !accounts[0] || !categories[0] || !session) return setNotice('Need budget/account/category'); await api('/transactions', session.token, { method: 'POST', body: JSON.stringify({ budget_id: activeBudget.id, account_id: accounts[0].id, date: txDate, payee: txPayee || null, memo: txMemo || null, splits: [{ category_id: categories[0].id, inflow: Number(txInflow), outflow: Number(txOutflow), memo: null }] }) }); setNotice('Transaction created'); setTxPayee(''); setTxMemo(''); setTxInflow('0'); setTxOutflow('0'); await refresh(); await refreshMonthProjection() }}><Input aria-label="Transaction date" type="date" value={txDate} onChange={(e) => setTxDate(e.target.value)} /><Input aria-label="Payee" value={txPayee} onChange={(e) => setTxPayee(e.target.value)} placeholder="Payee" /><Input aria-label="Memo" value={txMemo} onChange={(e) => setTxMemo(e.target.value)} placeholder="Memo" /><Input aria-label="Inflow" type="number" value={txInflow} onChange={(e) => setTxInflow(e.target.value)} /><Input aria-label="Outflow" type="number" value={txOutflow} onChange={(e) => setTxOutflow(e.target.value)} /><Button className="md:col-span-2">Create transaction</Button></form>{!transactions.length ? <p className="text-sm text-muted-foreground">No transactions yet</p> : <ul className="divide-y divide-white/10 border-y border-white/10">{transactions.slice(0, 8).map((tx) => <li key={tx.id} className="flex items-center justify-between px-1 py-2 text-sm"><span className="truncate">🧾 {tx.payee || 'Transaction'}</span><span className="font-semibold text-warning">{currency(tx.splits.reduce((acc, s) => acc + s.outflow - s.inflow, 0))}</span></li>)}</ul>}</section>}
      {activeTab === 'accounts' && <CrudPanel title="Accounts" items={accounts} parentRequired={!activeBudget} onCreate={async (name) => { if (!session || !activeBudget) return; await api('/accounts', session.token, { method: 'POST', body: JSON.stringify({ name, budget_id: activeBudget.id }) }); await refresh() }} />}
      {activeTab === 'settings' && <section className="space-y-4 border-t border-white/10 pt-3"><header><h2 className="font-brand text-2xl">Settings</h2></header><p className="text-sm text-muted-foreground">Manage authentication and session controls.</p><Button variant="secondary" onClick={() => { localStorage.removeItem('ez_session'); setSession(null) }}>Logout</Button></section>}
      {notice && <p className="mt-3 text-xs text-muted-foreground">{notice}</p>}
    </main>

    <aside className="workspace-pane hidden border-l border-white/10 px-4 pt-4 md:block">
      <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Inspector</p>
      <h3 className="mt-1 text-lg font-semibold">{selectedRow?.categoryName || 'Select a category'}</h3>
      <div className="mt-4 space-y-3 text-sm">
        <SummaryRow label="Assigned" value={currency(selectedRow?.assigned || 0)} />
        <SummaryRow label="Activity" value={currency(selectedRow?.activity || 0)} />
        <SummaryRow label="Available" value={currency(selectedRow?.available || 0)} highlighted />
      </div>
      <div className="mt-6 space-y-2">
        <Badge variant="outline">Inflow {currency(dashboard.inflow)}</Badge>
        <Badge variant="outline">Outflow {currency(dashboard.outflow)}</Badge>
      </div>
    </aside>

    <button className="mobile-cta md:hidden" onClick={() => setActiveTab('transactions')}><Plus className="h-4 w-4" /> Transaction</button>
    <nav className="mobile-nav md:hidden" aria-label="Mobile primary">
      {mobileNav.map((item, idx) => { const Icon = item.icon; const active = activeTab === item.id && idx !== 4; return <button key={`${item.label}-${idx}`} onClick={() => setActiveTab(item.id)} className={cn('mobile-nav-item', active && 'mobile-nav-item-active')}><Icon className="h-[18px] w-[18px]" /><span>{item.label}</span></button> })}
    </nav>
    <ToastViewport toasts={toasts} />
  </div>
}

function AvailabilityChip({ amount }: { amount: number }) {
  return <span className={cn('inline-flex min-w-[74px] justify-center rounded-full px-2 py-0.5 text-[11px] font-semibold', amount < 0 ? 'bg-rose-500/20 text-rose-200' : amount === 0 ? 'bg-slate-500/25 text-slate-300' : 'bg-emerald-500/20 text-emerald-200')}>{currency(amount)}</span>
}

function SummaryRow({ label, value, highlighted }: { label: string; value: string; highlighted?: boolean }) {
  return <div className="flex items-center justify-between border-b border-white/10 py-1.5 text-sm"><span className="text-muted-foreground">{label}</span><span className={cn('font-semibold', highlighted && 'text-primary')}>{value}</span></div>
}

function CrudPanel({ title, items, onCreate, parentRequired }: { title: string; items: any[]; onCreate: (name: string) => Promise<void>; parentRequired?: boolean }) {
  const [name, setName] = useState('')
  return <section className="space-y-3 border-t border-white/10 pt-3"><header><h2 className="font-brand text-2xl">{title}</h2></header><form className="space-y-2" onSubmit={async (e) => { e.preventDefault(); if (!name.trim() || parentRequired) return; await onCreate(name); setName('') }}><Input aria-label={`New ${title}`} value={name} onChange={(e) => setName(e.target.value)} disabled={parentRequired} /><Button disabled={parentRequired}>Create</Button></form>{!items.length && <p className="text-sm text-muted-foreground">No {title.toLowerCase()} yet</p>}<ul className="list-disc border-t border-white/10 pl-5 pt-3 text-sm">{items.map((x) => <li key={x.id}>{x.name}</li>)}</ul></section>
}

function ToastViewport({ toasts }: { toasts: Toast[] }) {
  return <div className="fixed right-4 top-4 z-50 grid gap-2" aria-live="polite">{toasts.map((toast) => <div className={cn('border border-white/10 px-3 py-2 text-sm text-white', toast.tone === 'error' && 'bg-rose-600/90', toast.tone === 'success' && 'bg-emerald-600/90', toast.tone === 'info' && 'bg-slate-700/90')} key={toast.id}>{toast.message}</div>)}</div>
}
