import { render, screen } from '@testing-library/react'
import { App } from './App'

describe('App', () => {
  it('renders primary login actions', () => {
    render(<App />)
    expect(screen.getByText('EnvelopeZero')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /send magic link/i })).toBeInTheDocument()
  })
})
