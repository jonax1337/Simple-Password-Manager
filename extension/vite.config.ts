import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";
import { copyFileSync, mkdirSync, existsSync } from "node:fs";

// Multi-entry MV3 build: popup is a React app, background and content are
// plain TS modules. We point Vite at all three and copy the static manifest
// into dist/ after each build.
export default defineConfig({
  plugins: [
    react(),
    {
      name: "copy-manifest",
      closeBundle() {
        const out = resolve(__dirname, "dist");
        if (!existsSync(out)) mkdirSync(out, { recursive: true });
        copyFileSync(resolve(__dirname, "manifest.json"), resolve(out, "manifest.json"));
        const icons = resolve(__dirname, "icons");
        if (existsSync(icons)) {
          mkdirSync(resolve(out, "icons"), { recursive: true });
          for (const f of ["icon16.png", "icon32.png", "icon48.png", "icon128.png"]) {
            const src = resolve(icons, f);
            if (existsSync(src)) {
              copyFileSync(src, resolve(out, "icons", f));
            }
          }
        }
      },
    },
  ],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        popup: resolve(__dirname, "popup.html"),
        background: resolve(__dirname, "src/background/index.ts"),
        content: resolve(__dirname, "src/content/index.ts"),
      },
      output: {
        entryFileNames: (chunk) => {
          if (chunk.name === "background") return "background.js";
          if (chunk.name === "content") return "content.js";
          return "assets/[name]-[hash].js";
        },
        chunkFileNames: "assets/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
    target: "es2022",
    minify: false, // makes review easier; flip to true for production
  },
});
