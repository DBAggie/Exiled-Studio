import { readFileSync } from 'node:fs'

import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

const pkg = JSON.parse(readFileSync('./package.json', 'utf8')) as {
  version: string
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // The app's version, for About (UX-22).
  define: { __APP_VERSION__: JSON.stringify(pkg.version) },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}', 'scripts/**/*.test.mjs'],
    // The screen tests walk whole workflows over the in-memory API: a
    // file's first test also pays for its imports, and on a busy machine
    // (or a CI runner) that passes the default five seconds.
    testTimeout: 20000,
  },
})
