import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite-plus";
import vue from "@vitejs/plugin-vue";
import UnoCSS from "unocss/vite";

export default defineConfig({
  fmt: {
    // src/bindings/** is emitted verbatim by ts-rs / tauri-specta on `pnpm run types:generate`
    // (i.e. `cargo test`), so formatting it would be undone by the next generation run.
    ignorePatterns: [".agents/**", "docs/**", "dist/**", "src-tauri/**", "src/bindings/**"],
  },
  lint: {
    ignorePatterns: [".agents/**", "docs/**", "dist/**", "src-tauri/**"],
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
  plugins: [vue(), UnoCSS()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: process.env.TAURI_DEV_HOST ?? "127.0.0.1",
    hmr: process.env.TAURI_DEV_HOST
      ? { protocol: "ws", host: process.env.TAURI_DEV_HOST, port: 1421 }
      : undefined,
  },
});
