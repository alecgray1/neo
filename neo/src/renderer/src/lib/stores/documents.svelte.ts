import { getMockContentString, mockFiles, type MockFile } from '../../mock-data'

export interface DocumentModel {
  id: string
  uri: string
  name: string
  content: string
  language: string
  version: number
  isDirty: boolean
  lastAccessed: number
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
  const models = $state<Map<string, DocumentModel>>(new Map())
  const mruOrder = $state<string[]>([]) // Most recently used URIs

  function countLines(content: string): number {
    return content.split('\n').length
  }

  function getLanguageFromUri(uri: string): string {
    if (uri.endsWith('.json')) return 'json'
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
      // Return existing if already open
      if (models.has(uri)) {
        this.touch(uri)
        return models.get(uri)!
      }

      // Get content based on URI scheme
      let content: string | null = null
      let name = uri.split('/').pop() ?? 'untitled'

      if (uri.startsWith('mock://')) {
        content = getMockContentString(uri)
        const mockFile = mockFiles.find((f) => f.uri === uri)
        if (mockFile) {
          name = mockFile.name
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

    // Update document content
    updateContent(uri: string, content: string): void {
      const doc = models.get(uri)
      if (!doc) return

      doc.content = content
      doc.version++
      doc.isDirty = true
      doc.metadata.lineCount = countLines(content)
      doc.metadata.size = new Blob([content]).size
      this.touch(uri)
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

    // Get mock files for file picker
    getMockFiles(): MockFile[] {
      return mockFiles
    }
  }
}

export const documentStore = createDocumentStore()
