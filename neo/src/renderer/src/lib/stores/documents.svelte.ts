import { SvelteMap } from 'svelte/reactivity'
import { serverStore } from './server.svelte'

export interface DocumentModel {
  id: string
  uri: string
  name: string
  content: string
  language: string
  version: number
  isDirty: boolean
  lastAccessed: number
  externalUpdateCounter: number // Increments when content is updated from server
  metadata: {
    size: number
    lineCount: number
    isLargeFile: boolean
  }
}

// Memory thresholds
const LARGE_FILE_THRESHOLD = 5 * 1024 * 1024 // 5MB
const LARGE_LINE_THRESHOLD = 100000 // 100K lines
const MAX_OPEN_DOCUMENTS = 50

function createDocumentStore() {
  const models = new SvelteMap<string, DocumentModel>()
  const mruOrder = $state<string[]>([]) // Most recently used URIs

  function countLines(content: string): number {
    return content.split('\n').length
  }

  function getLanguageFromUri(uri: string): string {
    // Special URIs for built-in editors
    if (uri.startsWith('keybindings://')) return 'keybindings'
    if (uri.startsWith('settings://')) return 'settings'

    // Neo server URIs
    if (uri.startsWith('neo://devices/')) return 'toml'
    if (uri.startsWith('neo://schedules/')) return 'toml'
    if (uri.startsWith('neo://blueprints/')) return 'blueprint'
    if (uri.startsWith('neo://bacnet/devices/')) return 'bacnet-device'

    // Blueprint files must be checked before generic .json
    if (uri.endsWith('.blueprint.json') || uri.endsWith('.bp.json')) return 'blueprint'
    if (uri.endsWith('.json')) return 'json'
    if (uri.endsWith('.toml')) return 'toml'
    if (uri.endsWith('.ts') || uri.endsWith('.tsx')) return 'typescript'
    if (uri.endsWith('.js') || uri.endsWith('.jsx')) return 'javascript'
    if (uri.endsWith('.svelte')) return 'svelte'
    if (uri.endsWith('.css')) return 'css'
    if (uri.endsWith('.html')) return 'html'
    if (uri.endsWith('.md')) return 'markdown'
    return 'plaintext'
  }

  return {
    get models() {
      return models
    },

    get mruOrder() {
      return mruOrder
    },

    // Get a document by URI
    get(uri: string): DocumentModel | undefined {
      return models.get(uri)
    },

    // Check if a document is open
    isOpen(uri: string): boolean {
      return models.has(uri)
    },

    // Open a document
    async open(uri: string): Promise<DocumentModel | null> {
      // For neo:// URIs, refresh content if not dirty (server data may have changed)
      if (uri.startsWith('neo://') && models.has(uri)) {
        const existing = models.get(uri)!
        if (!existing.isDirty) {
          // Refetch from server to get latest
          try {
            const data = await this.fetchNeoContent(uri)
            if (data !== null) {
              existing.content = data.content
              existing.version++
              existing.metadata.lineCount = countLines(data.content)
              existing.metadata.size = new Blob([data.content]).size
            }
          } catch (e) {
            console.error('Failed to refresh from Neo server:', e)
          }
        }
        this.touch(uri)
        return existing
      }

      // Return existing if already open (non-neo URIs)
      if (models.has(uri)) {
        this.touch(uri)
        return models.get(uri)!
      }

      // Get content based on URI scheme
      let content: string | null = null
      let name = uri.split('/').pop() ?? 'untitled'

      // Special URIs for built-in editors (no content needed)
      if (uri.startsWith('keybindings://')) {
        content = '' // Empty content, editor handles its own data
        name = 'Keyboard Shortcuts'
      } else if (uri.startsWith('settings://')) {
        content = ''
        name = 'Settings'
      } else if (uri.startsWith('neo://')) {
        // Fetch from Neo server
        try {
          const data = await this.fetchNeoContent(uri)
          if (data !== null) {
            content = data.content
            name = data.name
          }
        } catch (e) {
          console.error('Failed to fetch from Neo server:', e)
          return null
        }
      } else {
        // For real files, would use IPC to read from disk
        // For now, return null
        console.warn('Real file loading not implemented:', uri)
        return null
      }

      if (content === null) {
        console.error('Failed to load content for:', uri)
        return null
      }

      const lineCount = countLines(content)
      const size = new Blob([content]).size

      const doc: DocumentModel = {
        id: crypto.randomUUID(),
        uri,
        name,
        content,
        language: getLanguageFromUri(uri),
        version: 1,
        isDirty: false,
        lastAccessed: Date.now(),
        externalUpdateCounter: 0,
        metadata: {
          size,
          lineCount,
          isLargeFile: size > LARGE_FILE_THRESHOLD || lineCount > LARGE_LINE_THRESHOLD
        }
      }

      models.set(uri, doc)
      this.touch(uri)

      // Check if we need to prune old documents
      if (models.size > MAX_OPEN_DOCUMENTS) {
        this.pruneUnusedModels()
      }

      return doc
    },

    // Close a document
    close(uri: string): void {
      models.delete(uri)
      const idx = mruOrder.indexOf(uri)
      if (idx !== -1) {
        mruOrder.splice(idx, 1)
      }
    },

    // Update MRU tracking
    touch(uri: string): void {
      const idx = mruOrder.indexOf(uri)
      if (idx !== -1) {
        mruOrder.splice(idx, 1)
      }
      mruOrder.unshift(uri)

      const doc = models.get(uri)
      if (doc) {
        doc.lastAccessed = Date.now()
      }
    },

    // Get documents in MRU order
    getMRU(): DocumentModel[] {
      return mruOrder.map((uri) => models.get(uri)).filter((d): d is DocumentModel => d !== undefined)
    },

    // Get least recently used documents
    getLeastRecentlyUsed(count: number = 5): DocumentModel[] {
      return [...mruOrder]
        .reverse()
        .slice(0, count)
        .map((uri) => models.get(uri))
        .filter((d): d is DocumentModel => d !== undefined)
    },

    // Prune unused models when memory is high
    pruneUnusedModels(): void {
      if (models.size <= MAX_OPEN_DOCUMENTS) return

      // Get documents not currently being edited (not dirty)
      const pruneableUris = mruOrder.filter((uri) => {
        const doc = models.get(uri)
        return doc && !doc.isDirty
      })

      // Remove oldest first until under limit
      const toRemove = pruneableUris.slice(MAX_OPEN_DOCUMENTS - models.size)
      for (const uri of toRemove) {
        this.close(uri)
      }
    },

    // Update document content (and optionally sync to server)
    updateContent(uri: string, content: string, syncToServer: boolean = true): void {
      console.log('updateContent called for:', uri, 'syncToServer:', syncToServer)

      const doc = models.get(uri)
      if (!doc) {
        console.log('updateContent: document not found in models')
        return
      }

      doc.content = content
      doc.version++
      doc.isDirty = true
      doc.metadata.lineCount = countLines(content)
      doc.metadata.size = new Blob([content]).size
      this.touch(uri)

      // Sync to server for neo:// URIs
      if (syncToServer && uri.startsWith('neo://')) {
        console.log('updateContent: triggering syncToServer')
        this.syncToServer(uri, content)
      }
    },

    // Update content from server (external change) - doesn't sync back
    updateFromServer(uri: string, data: unknown): void {
      console.log('updateFromServer called for:', uri)
      const doc = models.get(uri)
      if (!doc) {
        console.log('updateFromServer: document not open, skipping')
        return // Document not open, nothing to update
      }

      const content = JSON.stringify(data, null, 2)

      // Only update if content actually changed and doc isn't dirty
      // (if dirty, user has local changes we don't want to overwrite)
      if (doc.content === content) {
        console.log('updateFromServer: content unchanged, skipping')
        return
      }
      if (doc.isDirty) {
        console.warn(`updateFromServer: Ignoring server update for ${uri} - document has local changes`)
        return
      }

      console.log('updateFromServer: updating document content')
      // Create a new document object to trigger Svelte reactivity
      const updatedDoc: DocumentModel = {
        ...doc,
        content,
        version: doc.version + 1,
        externalUpdateCounter: doc.externalUpdateCounter + 1,
        metadata: {
          ...doc.metadata,
          lineCount: countLines(content),
          size: new Blob([content]).size
        }
      }
      models.set(uri, updatedDoc)
      console.log(`updateFromServer: Document updated from server: ${uri}, externalUpdateCounter: ${updatedDoc.externalUpdateCounter}`)
    },

    // Sync content to the server via WebSocket
    async syncToServer(uri: string, content: string): Promise<boolean> {
      const match = uri.match(/^neo:\/\/(\w+)\/(.+)$/)
      if (!match) return false

      const [, type, id] = match

      try {
        // Parse content back to object
        const data = JSON.parse(content)

        console.log('syncToServer: sending data for', uri)
        console.log('syncToServer: nodes positions:', data.nodes?.map((n: any) => ({ id: n.id, x: n.position?.x, y: n.position?.y })))

        // Send Update message via WebSocket
        const path = `/${type}/${id}`
        await window.serverAPI.request(path, { action: 'update', data })

        // Mark as saved
        this.markSaved(uri)
        console.log(`Synced ${uri} to server - SUCCESS`)
        return true
      } catch (e) {
        console.error('Failed to sync to server:', e)
        return false
      }
    },

    // Mark document as saved
    markSaved(uri: string): void {
      const doc = models.get(uri)
      if (doc) {
        doc.isDirty = false
      }
    },

    // Get total memory usage estimate
    getMemoryUsage(): number {
      let total = 0
      for (const doc of models.values()) {
        total += doc.metadata.size
      }
      return total
    },

    // Get all open document URIs
    getOpenUris(): string[] {
      return [...models.keys()]
    },

    // Fetch content from Neo server
    async fetchNeoContent(uri: string): Promise<{ content: string; name: string } | null> {
      // Parse neo:// URI: neo://type/id
      const match = uri.match(/^neo:\/\/(\w+)\/(.+)$/)
      if (!match) {
        console.error('Invalid neo:// URI:', uri)
        return null
      }

      const [, type, id] = match

      try {
        let data: unknown
        let name: string

        switch (type) {
          case 'devices': {
            const device = serverStore.getDevice(id)
            if (!device) {
              // Fetch from server if not in cache
              data = await window.serverAPI.request(`/devices/${id}`)
            } else {
              data = device
            }
            name = `${id}.device.toml`
            break
          }
          case 'blueprints': {
            // Always fetch fresh from server to get latest changes
            data = await window.serverAPI.request(`/blueprints/${id}`)
            console.log('fetchNeoContent: fetched blueprint from server:', id, data)
            name = `${id}.bp.json`
            break
          }
          case 'schedules': {
            const schedule = serverStore.getSchedule(id)
            if (!schedule) {
              data = await window.serverAPI.request(`/schedules/${id}`)
            } else {
              data = schedule
            }
            name = `${id}.schedule.toml`
            break
          }
          case 'bacnet': {
            // Handle neo://bacnet/devices/{id}
            const subPath = id // e.g., "devices/101"
            if (subPath.startsWith('devices/')) {
              const deviceId = subPath.replace('devices/', '')
              data = await window.serverAPI.request(`/bacnet/devices/${deviceId}`)
              name = `Device ${deviceId}`
            } else {
              console.error('Unknown bacnet sub-path:', subPath)
              return null
            }
            break
          }
          default:
            console.error('Unknown neo:// type:', type)
            return null
        }

        if (!data) {
          return null
        }

        // Format content based on type
        const content = JSON.stringify(data, null, 2)
        return { content, name }
      } catch (e) {
        console.error('Failed to fetch neo content:', e)
        return null
      }
    }
  }
}

export const documentStore = createDocumentStore()
