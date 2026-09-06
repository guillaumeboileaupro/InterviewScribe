import { configDefaults, defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./apps/client/src/test/setup.ts"],
    // scripts/*.test.mjs uses Node's native test runner (see "test:packaging"
    // in package.json), not Vitest — exclude it so `pnpm test` only picks up
    // the frontend suite.
    exclude: [...configDefaults.exclude, "scripts/**"],
  },
});
