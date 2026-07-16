import {
  defineConfig,
  presetIcons,
  presetUno,
  transformerDirectives,
  transformerVariantGroup,
} from "unocss";

export default defineConfig({
  presets: [presetUno(), presetIcons()],
  transformers: [transformerDirectives(), transformerVariantGroup()],
  shortcuts: {
    "surface-card":
      "rounded-xl border border-slate-200 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-950",
    "focus-ring":
      "outline-none ring-2 ring-sky-500 ring-offset-2 ring-offset-white dark:ring-offset-slate-950",
  },
  theme: {
    colors: {
      brand: "#0ea5e9",
    },
  },
});
