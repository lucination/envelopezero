import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './scripts',
  testMatch: /e2e\.spec\.ts/,
  timeout: 90_000,
  use: {
    baseURL: process.env.EZ_APP_URL || 'http://127.0.0.1:8080',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
})
