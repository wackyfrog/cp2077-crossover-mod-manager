import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { readFileSync } from 'fs'
import { execSync } from 'child_process'

const { version } = JSON.parse(readFileSync('./package.json', 'utf-8'))

// Commit the build was made from, e.g. "f6611d3" or "f6611d3-dirty" when
// tracked files had uncommitted changes. Empty outside a git checkout.
const commit = (() => {
  try {
    const hash = execSync('git rev-parse --short HEAD', { encoding: 'utf-8' }).trim()
    let dirty = false
    try { execSync('git diff --quiet HEAD') } catch { dirty = true }
    return dirty ? `${hash}-dirty` : hash
  } catch {
    return ''
  }
})()

// https://vitejs.dev/config/
export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(version),
    __GIT_COMMIT__: JSON.stringify(commit),
    __BUILD_ID__: (() => {
      const now = new Date();
      const pad = (n) => String(n).padStart(2, '0');
      return JSON.stringify(`${pad(now.getMonth()+1)}${pad(now.getDate())}*${pad(now.getHours())}${pad(now.getMinutes())}`);
    })(),
  },
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: process.env.TAURI_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})
