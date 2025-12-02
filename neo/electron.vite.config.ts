import { defineConfig, externalizeDepsPlugin } from 'electron-vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'path'

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
    build: {
      rollupOptions: {
        input: {
          index: resolve(__dirname, 'src/main/index.ts'),
          // Extension host process - separate entry for forking
          ExtensionHostProcess: resolve(__dirname, 'src/main/extensionHost/ExtensionHostProcess.ts')
        }
      }
    }
  },
  preload: {
    plugins: [externalizeDepsPlugin()]
  },
  renderer: {
    resolve: {
      alias: {
        $lib: resolve('src/renderer/src/lib')
      }
    },
    plugins: [svelte(), tailwindcss()]
  }
})
