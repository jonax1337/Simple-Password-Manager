import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// Outputs a SPA suitable for serving from any static host. In our
// docker-compose setup, Caddy serves dist/ at /dashboard/* alongside the API.
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: true,
  },
});
