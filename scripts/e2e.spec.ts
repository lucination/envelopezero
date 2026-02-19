import { test, expect } from '@playwright/test'

const appUrl = process.env.EZ_APP_URL || 'http://127.0.0.1:8080'
const mailpitUrl = process.env.EZ_MAILPIT_URL || 'http://127.0.0.1:8025'
const email = `adi+${Date.now()}@example.com`

async function getMagicToken() {
  const res = await fetch(`${mailpitUrl}/api/v1/messages`)
  const data = await res.json()
  const matches = (data.messages || []).filter((m: any) => (m.To || []).some((t: any) => t.Address === email))
  const msg = matches.sort((a: any, b: any) => Date.parse(b.Created) - Date.parse(a.Created))[0]
  if (!msg?.ID) return ''
  const fullRes = await fetch(`${mailpitUrl}/api/v1/message/${msg.ID}`)
  const full = await fullRes.json()
  const body: string = full.Text || full.HTML || ''
  const match = body.match(/token=([A-Za-z0-9_-]+)/)
  return match?.[1] || ''
}

test('MVP full browser flow', async ({ page }) => {
  await page.goto(appUrl)
  await page.evaluate(() => localStorage.clear())
  await page.reload()
  if (await page.getByRole('button', { name: 'Logout' }).isVisible().catch(() => false)) {
    await page.getByRole('button', { name: 'Logout' }).click()
  }

  await page.getByLabel('Email').fill(email)
  await page.getByRole('button', { name: 'Send magic link' }).click()

  await expect.poll(async () => getMagicToken(), { timeout: 15000 }).toMatch(/[A-Za-z0-9_-]{20,}/)
  const token = await getMagicToken()

  await page.getByLabel('Token').fill(token)
  await expect(page.getByText('Dashboard (USD cents)')).toBeVisible()
  await expect(page.getByLabel('New Accounts')).toBeEnabled()

  await expect(page.getByText('No transactions yet')).toBeVisible()

  // Multi-budget gate
  await page.getByLabel('New Budgets').fill('Another Budget')
  await page.getByRole('button', { name: 'Create' }).first().click()
  await expect(page.getByText('Multi-budget feature is disabled for MVP')).toBeVisible()

  // Accounts CRUD
  await page.getByLabel('New Accounts').fill('Cash')
  await page.getByLabel('New Accounts').press('Enter')
  await expect(page.getByText('Cash')).toBeVisible()
  await page.locator('section:has-text("Accounts") li:has-text("Cash") button:has-text("Edit")').click()
  await page.getByLabel('New Accounts').fill('Wallet')
  await page.getByLabel('New Accounts').press('Enter')
  await expect(page.getByText('Wallet')).toBeVisible()

  // Supercategories CRUD
  await page.getByLabel('New Supercategories').fill('Needs')
  await page.getByLabel('New Supercategories').press('Enter')
  await expect(page.getByText('Needs')).toBeVisible()

  // Categories CRUD + dependency gate implicitly cleared
  await page.getByLabel('New Categories').fill('Groceries')
  await page.getByLabel('New Categories').press('Enter')
  await expect(page.getByText('Groceries')).toBeVisible()

  // Transaction CRUD + dashboard update
  await page.getByLabel('Payee').fill('Trader Joes')
  await page.getByLabel('Outflow').fill('2500')
  await page.getByRole('button', { name: 'Create transaction' }).click()
  await expect(page.getByText('Trader Joes')).toBeVisible()
  await expect(page.getByText('Outflow: 2500')).toBeVisible()

  await page.locator('section:has-text("Transactions") li:has-text("Trader Joes") button:has-text("Edit")').click()
  await page.getByLabel('Outflow').fill('3000')
  await page.getByRole('button', { name: 'Update transaction' }).click()
  await expect(page.getByText('Outflow: 3000')).toBeVisible()

  await page.locator('section:has-text("Transactions") li:has-text("Trader Joes") button:has-text("Delete")').click()
  await expect(page.getByText('No transactions yet')).toBeVisible()

  // Session persistence + logout
  await page.reload()
  await expect(page.getByText('Dashboard (USD cents)')).toBeVisible()

  await page.getByRole('button', { name: 'Logout' }).click()
  await expect(page.getByRole('button', { name: 'Send magic link' })).toBeVisible()

  await page.setViewportSize({ width: 390, height: 844 })
  await expect(page.getByRole('button', { name: 'Send magic link' })).toBeVisible()
})
