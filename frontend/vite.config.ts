import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:3579/',
      '/levels': 'http://127.0.0.1:3579/',
      '/socket.io': {
        target: 'ws://127.0.0.1:3579/',
        ws: true,
      },
    },
  },
});
