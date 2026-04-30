import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(
  readFileSync(resolve(__dirname, "package.json"), "utf8"),
) as { version: string };

// Tauri serves the bundled assets via its own dev server on a fixed
// port — Vite has to use the matching port + dev URL to play nicely
// with Tauri's hot reload. Tauri reads the port from this config
// via `frontendDist` / `devUrl` in tauri.conf.json.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  define: {
    // Surfaces the package version to the React frontend so the
    // footer can show "v0.1.0" without the user having to update it
    // in two places.
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
});
