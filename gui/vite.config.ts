import { resolve } from "node:path";
import tailwind from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The UI of the app: the web view of Tauri loads it from the dev server or from dist, and a browser loads it too, on
// the host that the address of the page names. The model of a life comes from its source in bind/typescript, which
// holds no native code, so the UI needs no build of the package.
export default defineConfig({
  plugins: [react(), tailwind()],
  resolve: {
    alias: {
      "furb/life": resolve(import.meta.dirname, "../bind/typescript/src/life/index.ts"),
    },
  },
  clearScreen: false,
  server: { port: 5174, strictPort: true },
  preview: { port: 4174, strictPort: true },
  build: { target: "safari16", outDir: "dist", chunkSizeWarningLimit: 2048 },
});
