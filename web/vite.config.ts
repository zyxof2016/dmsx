import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": resolve(__dirname, "src") },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) return;

          if (
            id.includes("/react/") ||
            id.includes("/react-dom/") ||
            id.includes("/scheduler/")
          ) {
            return "react-vendor";
          }
          if (id.includes("livekit-client")) {
            return "livekit";
          }
          if (id.includes("recharts")) {
            return "charts";
          }
          if (
            id.includes("/antd/") ||
            id.includes("/@ant-design/") ||
            id.includes("/rc-") ||
            id.includes("/@rc-component/") ||
            id.includes("/stylis/") ||
            id.includes("/@emotion/") ||
            id.includes("/compute-scroll-into-view/") ||
            id.includes("/scroll-into-view-if-needed/") ||
            id.includes("/resize-observer-polyfill/")
          ) {
            return "antd-vendor";
          }
          if (
            id.includes("@tanstack/react-query") ||
            id.includes("@tanstack/react-router")
          ) {
            return "tanstack";
          }
          if (id.includes("/dayjs/")) {
            return "dayjs";
          }

          return "vendor";
        },
      },
    },
  },
  server: {
    port: 3000,
    proxy: {
      "/v1": "http://127.0.0.1:8080",
      "/health": "http://127.0.0.1:8080",
    },
  },
});
