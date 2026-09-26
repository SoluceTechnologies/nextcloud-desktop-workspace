import {defineConfig} from "vite";
import react from "@vitejs/plugin-react";
// @ts-expect-error type error without @types/node package
import process from "node:process";
// @ts-expect-error type error without @types/node package
import {fileURLToPath} from "node:url";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
    plugins: [react()],
    resolve: {
        alias: {"@": fileURLToPath(new URL("./src", import.meta.url))},
    },
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
            ? {
                protocol: "ws",
                host,
                port: 1421,
            }
            : undefined,
        watch: {
            ignored: ["**/src-tauri/**"],
        },
    },
}));
