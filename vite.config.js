import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

/**
 * vite-plugin-svelte serves a component's CSS from the result of compiling the
 * component. If the webview asks for the CSS first (e.g. JS served from its
 * cache), that result doesn't exist yet: Vite then caches the raw .svelte file
 * as CSS and the component stays unstyled until restart ("failed to load
 * virtual css module"). Compile the component first in that case.
 * @returns {import("vite").Plugin}
 */
function compileSvelteBeforeCss() {
  /** @type {import("vite").ViteDevServer | undefined} */
  let server;
  return {
    name: "compile-svelte-before-css",
    apply: "serve",
    enforce: "pre",
    configureServer(s) {
      server = s;
    },
    async load(id) {
      const match = /^(.+\.svelte)\?svelte&type=style/.exec(id);
      if (!server || !match || this.getModuleInfo(match[1])?.meta?.svelte?.css) return null;
      const file = match[1];
      const url = file.startsWith(server.config.root + "/")
        ? file.slice(server.config.root.length)
        : `/@fs${file}`;
      await server.environments[this.environment.name].transformRequest(url);
      return null; // let vite-plugin-svelte serve the CSS
    },
  };
}

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [compileSvelteBeforeCss(), sveltekit()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
