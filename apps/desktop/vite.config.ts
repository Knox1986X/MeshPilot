import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420, strictPort: true,
    // Привязка к внешнему интерфейсу только по флагу --host для предпросмотра.
    allowedHosts: [".e2b.app"],
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
