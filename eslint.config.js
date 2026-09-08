import js from "@eslint/js";
import { defineConfig, globalIgnores } from "eslint/config";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import { reactRefresh } from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

export default defineConfig([
  // .codex/.claude/.agents are local, gitignored agent working directories
  // (see CLAUDE.md) - excluding them isn't just correctness (no TS/TSX
  // source lives there), it also stops ESLint's glob walk from racing
  // another agent concurrently writing to its own directory, which was
  // intermittently crashing `pnpm check` with ENOENT.
  globalIgnores(["dist", "src-tauri", ".codex", ".claude", ".agents"]),
  {
    files: ["**/*.{ts,tsx}"],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite(),
    ],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
  },
]);
