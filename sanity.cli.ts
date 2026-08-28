import { defineCliConfig } from "sanity/cli";

export default defineCliConfig({
  typegen: {
    path: "./src/**/*.{ts,tsx,js,jsx}",
    schema: "./studio-aizawa/schema.json",
    generates: "./src/sanity.types.ts",
  },
});
