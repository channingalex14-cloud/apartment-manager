import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import Components from "unplugin-vue-components/vite";
import AutoImport from "unplugin-auto-import/vite";
import { resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      imports: ["vue", "vue-router", "pinia"],
      dts: "src/auto-imports.d.ts",
      vueTemplate: true,
    }),
    Components({
      dts: "src/components.d.ts",
      resolvers: [
        (name) => {
          // element-plus icon components only (icons are named: HomeFilled, User, etc.)
          const iconNames = [
            'HomeFilled', 'OfficeBuilding', 'User', 'Document', 'Tickets', 'Money',
            'Coin', 'Setting', 'Fold', 'Expand', 'Search', 'Refresh', 'Plus',
            'Minus', 'Close', 'Check', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown',
            'Delete', 'Edit', 'View', 'More', 'MoreFilled', 'Info', 'Warning',
            'Success', 'Error', 'Question', 'InfoFilled', 'WarningFilled',
            'CircleCheck', 'CircleClose', 'CloseBold', 'PlusBold', 'EditPen',
            'DeleteFilled', 'DeletePen', 'Search', 'Refresh', 'RefreshLeft', 'RefreshRight',
            'House', 'Wallet', 'DataAnalysis', 'TrendCharts', 'Lock', 'Folder',
            'Tickets', 'Odometer',
          ];
          if (iconNames.includes(name)) {
            return {
              name,
              from: "@element-plus/icons-vue",
              sideEffect: false,
            };
          }
          // Other El* components come from element-plus
          if (name.startsWith("El")) {
            return {
              name,
              from: "element-plus",
              sideEffect: false,
            };
          }
        },
      ],
    }),
  ],
  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1423,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1422,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: ["es2021", "chrome100", "safari13"],
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      output: {
        manualChunks: {
          echarts: ['echarts'],
          'element-plus': ['element-plus'],
        },
      },
    },
  },
});
