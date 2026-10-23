import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri's dev server expects a fixed port; the browser preview uses the same one.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
});
