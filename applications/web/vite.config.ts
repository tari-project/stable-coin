import {defineConfig} from "vite";
import react from "@vitejs/plugin-react-swc";
// import {NodeGlobalsPolyfillPlugin} from "@esbuild-plugins/node-globals-polyfill";
// import {NodeModulesPolyfillPlugin} from "@esbuild-plugins/node-modules-polyfill";

// https://vitejs.dev/config/
export default defineConfig({
    plugins: [react()],
    // These are polyfills to get @metamask/provider to work in the browser.
    // https://gist.github.com/FbN/0e651105937c8000f10fefdf9ec9af3d
    resolve: {
        alias: {
            // stream: "rollup-plugin-node-polyfills/polyfills/stream",
            // events: "rollup-plugin-node-polyfills/polyfills/events",
            // util: "rollup-plugin-node-polyfills/polyfills/util",
        },
    },
    optimizeDeps: {
        esbuildOptions: {
            // Node.js global to browser globalThis
            define: {
                global: "globalThis",
            },
            // Enable esbuild polyfill plugins
            plugins: [
                // NodeGlobalsPolyfillPlugin({
                //     process: true,
                //     buffer: true,
                // }),
                // NodeModulesPolyfillPlugin(),
            ],
        },
    },
});
