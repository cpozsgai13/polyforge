import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    // Proxy API calls to the Rust server so the browser never sees
    // cross-origin requests (avoids CORS and self-signed cert issues in dev).
    proxy: {
      '/trading': {
        target: 'https://localhost:8443',
        changeOrigin: true,
        secure: false,   // accept self-signed cert
      },
      '/market': {
        target: 'https://localhost:8443',
        changeOrigin: true,
        secure: false,
      },
      '/api': {
        target: 'https://localhost:8443',
        changeOrigin: true,
        secure: false,
      },
    },
  },
})
