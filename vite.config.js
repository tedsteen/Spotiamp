import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static to prerender the app (SSG).
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [sveltekit({ adapter: adapter() })],
  build: {
    target: ["chrome87", "edge88", "safari14"],
    cssTarget: ["chrome87", "edge88", "safari14"],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
});
