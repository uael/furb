import tailwind from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The UI of the app: the web view of Tauri loads it from the dev server or from dist, and a browser loads it too, on
// the view of the demo that src/data/demo.json holds.
export default defineConfig({
  plugins: [react(), tailwind()],
  clearScreen: false,
  server: { port: 5174, strictPort: true },
  preview: { port: 4174, strictPort: true },
  build: { target: "safari16", outDir: "dist" },
});
