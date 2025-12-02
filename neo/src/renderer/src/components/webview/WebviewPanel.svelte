<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import type { WebviewInstance } from '$lib/webview/WebviewService'

  interface Props {
    webview: WebviewInstance
  }

  let { webview }: Props = $props()

  let iframeRef: HTMLIFrameElement | null = $state(null)

  // Security: Generate a nonce for CSP
  const nonce = crypto.randomUUID()

  // Create a sandboxed blob URL for the webview content
  function createBlobUrl(html: string): string {
    // Wrap the HTML with security headers and neo API bridge
    const wrappedHtml = `
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}'; img-src data: https:; font-src data:;">
  <style>
    body {
      margin: 0;
      padding: 8px;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      font-size: 13px;
      color: var(--neo-foreground, #ccc);
      background: var(--neo-background, #1e1e1e);
    }
    :root {
      color-scheme: dark;
    }
  </style>
  <script nonce="${nonce}">
    // Webview API bridge
    const neoWebview = {
      postMessage: function(message) {
        window.parent.postMessage({ type: 'webview-message', handle: '${webview.handle}', message }, '*');
      },
      onMessage: function(callback) {
        window.addEventListener('message', function(e) {
          if (e.data && e.data.type === 'extension-message') {
            callback(e.data.message);
          }
        });
      }
    };
    window.neoWebview = neoWebview;
  </script>
</head>
<body>
${html}
</body>
</html>
`
    const blob = new Blob([wrappedHtml], { type: 'text/html' })
    return URL.createObjectURL(blob)
  }

  let blobUrl = $derived(createBlobUrl(webview.html))

  // Handle messages from the webview
  function handleMessage(event: MessageEvent) {
    if (event.data?.type === 'webview-message' && event.data.handle === webview.handle) {
      // Forward message to extension host via IPC
      // This would go through the main process
      console.log('[WebviewPanel] Message from webview:', event.data.message)
    }
  }

  onMount(() => {
    window.addEventListener('message', handleMessage)
  })

  onDestroy(() => {
    window.removeEventListener('message', handleMessage)
    if (blobUrl) {
      URL.revokeObjectURL(blobUrl)
    }
  })

  // Update blob URL when html changes
  $effect(() => {
    if (iframeRef && webview.html) {
      const newUrl = createBlobUrl(webview.html)
      // Clean up old URL after iframe loads new one
      const oldUrl = iframeRef.src
      iframeRef.src = newUrl
      if (oldUrl && oldUrl.startsWith('blob:')) {
        setTimeout(() => URL.revokeObjectURL(oldUrl), 100)
      }
    }
  })
</script>

<div class="webview-container">
  <iframe
    bind:this={iframeRef}
    src={blobUrl}
    sandbox="allow-scripts allow-forms"
    title={webview.title}
    class="webview-frame"
  ></iframe>
</div>

<style>
  .webview-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .webview-frame {
    width: 100%;
    height: 100%;
    border: none;
    background: var(--neo-background);
  }
</style>
