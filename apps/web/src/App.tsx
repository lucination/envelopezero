import { useState } from 'react'

export function App() {
  const [email, setEmail] = useState('')
  const [status, setStatus] = useState('')

  async function requestMagicLink(e: React.FormEvent) {
    e.preventDefault()
    setStatus('Sending…')

    try {
      const res = await fetch('http://localhost:8080/auth/magic-link/request', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email }),
      })

      if (!res.ok) throw new Error('Request failed')
      const data = await res.json()
      setStatus(data.message)
    } catch {
      setStatus('Could not request magic link right now.')
    }
  }

  return (
    <main className="shell">
      <section className="card">
        <h1>EnvelopeZero</h1>
        <p className="sub">Budgeting without passwords.</p>

        <form onSubmit={requestMagicLink}>
          <label htmlFor="email">Email</label>
          <input
            id="email"
            type="email"
            placeholder="you@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
          <button type="submit">Send magic link</button>
        </form>

        <button className="secondary" disabled>
          Use passkey (coming next)
        </button>

        {status && <p className="status">{status}</p>}
      </section>
    </main>
  )
}
