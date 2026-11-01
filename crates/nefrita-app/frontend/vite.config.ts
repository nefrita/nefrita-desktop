import { createHash } from "node:crypto";
import createReactPlugins from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const developmentServerPort = 5173;

const reactRefreshPreambleBasePlaceholder = "__BASE__";
if (!createReactPlugins.preambleCode.includes(reactRefreshPreambleBasePlaceholder)) {
  throw new Error("The preamble of @vitejs/plugin-react has no __BASE__ placeholder");
}

const reactRefreshPreamble = createReactPlugins.preambleCode.replace(
  reactRefreshPreambleBasePlaceholder,
  "/",
);
const reactRefreshPreambleHash = createHash("sha256").update(reactRefreshPreamble).digest("base64");

const developmentContentSecurityPolicy = [
  "default-src 'self'",
  `script-src 'self' 'sha256-${reactRefreshPreambleHash}'`,
  "style-src 'self' 'unsafe-inline'",
  `connect-src 'self' ws://localhost:${developmentServerPort}`,
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
].join("; ");

const developmentContentSecurityPolicyPlugin: Plugin = {
  name: "development-content-security-policy",
  apply: "serve",
  transformIndexHtml: {
    order: "post",
    handler: () => [
      {
        tag: "meta",
        attrs: {
          "http-equiv": "Content-Security-Policy",
          content: developmentContentSecurityPolicy,
        },
        injectTo: "head-prepend",
      },
    ],
  },
};

export default defineConfig({
  plugins: [createReactPlugins(), developmentContentSecurityPolicyPlugin],
  clearScreen: false,
  server: {
    port: developmentServerPort,
    strictPort: true,
  },
  build: {
    assetsInlineLimit: 0,
  },
});
