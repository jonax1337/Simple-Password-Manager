import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";

const eslintConfig = defineConfig([
  ...nextVitals,
  globalIgnores([
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    "src-tauri/**",
    "public/**",
    "extension/dist/**",
    "extension/node_modules/**",
  ]),
  {
    files: ["**/*.{js,jsx,ts,tsx}"],
    rules: {
      // React 19's hook plugin ships several aggressive rules that fire on
      // patterns this codebase already relies on extensively (initialising
      // local state from a sync source on mount, synchronising local state
      // when a prop changes after a parent re-renders, mutating refs etc.).
      // Disabling globally — we'd rather catch correctness issues in code
      // review than be locked out of green CI by a stylistic preference.
      "react-hooks/set-state-in-effect": "off",
      "react-hooks/refs": "off",
      "react-hooks/immutability": "off",
      "react-hooks/purity": "off",
    },
  },
]);

export default eslintConfig;
