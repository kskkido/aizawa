// @ts-check

import sitemap from "@astrojs/sitemap";
import sanity from "@sanity/astro";
import { defineConfig } from "astro/config";

// https://astro.build/config
export default defineConfig({
  site: process.env.SITE,
  prefetch: {
    prefetchAll: true,
    defaultStrategy: "viewport",
  },
  experimental: {
    clientPrerender: true,
  },
  integrations: [
    sanity({
      projectId: process.env.SANITY_STUDIO_PROJECT_ID,
      dataset: process.env.SANITY_STUDIO_DATASET,
    }),
    sitemap(),
  ],
  vite: {
    resolve: {
      alias: {
        "@": new URL("./src", import.meta.url).pathname,
        "@design-system": new URL(
          "../../packages/design-system",
          import.meta.url,
        ).pathname,
      },
    },
  },
});
