import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri dev server: fixed port, strict
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2021",
    minify: false,
    sourcemap: true,
  },
});
