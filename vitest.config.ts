import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

// Vitest runs against the same `@/*` path alias the Next.js build uses,
// so test files can import the real lib/* and hooks/* modules unmodified.
// `extension/` has its own test setup (none today) and is excluded here
// to avoid Vitest scooping up the embedded TS files.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    include: ["tests/frontend/**/*.test.ts", "tests/frontend/**/*.test.tsx"],
    exclude: ["node_modules", "src-tauri", "extension", ".next", "out"],
  },
});
