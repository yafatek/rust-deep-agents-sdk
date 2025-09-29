import { defineConfig } from 'vite'

export default defineConfig({
  server: {
    port: 5173,
    host: true,
    cors: true
  },
  build: {
    outDir: 'dist',
    assetsDir: 'assets'
  }
})
