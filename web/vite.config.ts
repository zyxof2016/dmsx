import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

function getAntdChunkName(id: string): string {
  if (id.includes("/node_modules/antd/")) {
    return "antd-core";
  }

  const antDesignMatch = id.match(/\/node_modules\/(@ant-design(?:\/icons)?|@rc-component\/[^/]+|rc-[^/]+)\//);
  if (antDesignMatch) {
    const packageName = antDesignMatch[1];
    if (packageName === "@ant-design/icons") {
      return "antd-icons";
    }
    if (
      packageName === "@ant-design" ||
      packageName.startsWith("@rc-component/") ||
      packageName.startsWith("rc-")
    ) {
      return "antd-framework";
    }
    return `antd-${packageName.replace(/[\/]/g, "-")}`;
  }

  if (id.includes("/node_modules/@emotion/")) {
    return "antd-emotion";
  }
  if (id.includes("/node_modules/stylis/")) {
    return "antd-stylis";
  }
  if (
    id.includes("/node_modules/compute-scroll-into-view/") ||
    id.includes("/node_modules/scroll-into-view-if-needed/")
  ) {
    return "antd-scroll";
  }
  if (id.includes("/node_modules/resize-observer-polyfill/")) {
    return "antd-resize-observer";
  }

  return "antd-shared";
}

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
            return getAntdChunkName(id);
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
