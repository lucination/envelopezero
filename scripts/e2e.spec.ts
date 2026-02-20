import { test, expect } from '@playwright/test'

const appUrl = process.env.EZ_APP_URL || 'http://127.0.0.1:8080'

test('core flow with projection and coherent UI', async ({ page }) => {
  const email = `adi+${Date.now()}@example.com`
  const token = await page.request.post(`${appUrl}/api/auth/magic-link/request`, { data: { email } }).then((r) => r.json()).then((j) => j.debug_token as string)
  const session = await page.request.post(`${appUrl}/api/auth/magic-link/verify`, { data: { token } }).then((r) => r.json())
  const auth = { Authorization: `Bearer ${session.token}` }
  const budgetId = await page.request.get(`${appUrl}/api/budgets`, { headers: auth }).then((r) => r.json()).then((rows) => rows[0].id as string)

  await page.request.post(`${appUrl}/api/accounts`, { headers: { ...auth, 'Content-Type': 'application/json' }, data: { name: 'Checking', budget_id: budgetId } })
  const superId = await page.request.post(`${appUrl}/api/supercategories`, { headers: { ...auth, 'Content-Type': 'application/json' }, data: { name: 'Needs', budget_id: budgetId } }).then((r) => r.json()).then((j) => j.id as string)
  await page.request.post(`${appUrl}/api/categories`, { headers: { ...auth, 'Content-Type': 'application/json' }, data: { name: 'Groceries', budget_id: budgetId, supercategory_id: superId } })

  await page.goto(appUrl)
  await page.evaluate((s) => localStorage.setItem('ez_session', JSON.stringify(s)), session)
  await page.reload()

  await expect(page.getByRole('heading', { name: 'Budget' })).toBeVisible()
  await expect(page.getByTestId('ready-to-assign')).toBeVisible()
  await page.screenshot({ path: 'docs/assets/ui-shell-desktop.png', fullPage: true })
  await page.screenshot({ path: 'docs/assets/ui-budget-workspace.png', fullPage: true })

  await page.getByRole('button', { name: 'Add transaction' }).click()
  await page.getByLabel('Payee').fill('Market')
  await page.getByLabel('Outflow').fill('2500')
  await page.getByRole('button', { name: 'Create transaction' }).click()

  await page.setViewportSize({ width: 390, height: 844 })
  await page.screenshot({ path: 'docs/assets/ui-shell-mobile.png', fullPage: true })
})
