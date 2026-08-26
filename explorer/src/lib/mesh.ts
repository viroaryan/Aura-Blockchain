import type Peer from 'peerjs'
import type { DataConnection } from 'peerjs'

export type MediaType = 'text' | 'image' | 'video' | 'audio' | 'document'

export interface PeerMessage {
  id: string
  sender: 'self' | 'peer'
  text?: string
  timestamp: number
  type: 'chat' | 'file_meta' | 'file_chunk' | 'ping' | 'pong' | 'speed_chunk' | 'voice_note'
  mediaType?: MediaType
  audioUrl?: string
  fileMeta?: {
    fileId: string
    fileName: string
    fileSize: number
    fileType: string
    mediaType: MediaType
    chunksTotal: number
  }
}

export interface FileTransferProgress {
  id: string
  fileName: string
  fileSize: number
  fileType: string
  mediaType: MediaType
  receivedBytes: number
  progressPercent: number
  isComplete: boolean
  downloadUrl?: string
  previewUrl?: string
  sender: 'self' | 'peer'
}

export interface LiveMeshDiagnostics {
  pingMs: number
  bytesSent: number
  bytesReceived: number
  realSpeedMbps: number
  connectionState: 'disconnected' | 'connecting' | 'connected'
  remotePeerId?: string
}

export function detectMediaType(mimeType: string, filename: string): MediaType {
  const lowerMime = mimeType.toLowerCase()
  const lowerName = filename.toLowerCase()

  if (lowerMime.startsWith('image/') || /\.(jpg|jpeg|png|gif|webp|svg)$/i.test(lowerName)) {
    return 'image'
  }
  if (lowerMime.startsWith('video/') || /\.(mp4|webm|mkv|mov|avi)$/i.test(lowerName)) {
    return 'video'
  }
  if (lowerMime.startsWith('audio/') || /\.(mp3|wav|ogg|m4a|aac|webm)$/i.test(lowerName)) {
    return 'audio'
  }
  return 'document'
}

export class RealAuraMeshEngine {
  private peer: Peer | null = null
  private connection: DataConnection | null = null
  private myPeerId: string = ''
  private heartbeatInterval: any = null

  private bytesSent = 0
  private bytesReceived = 0
  private lastPingTime = 0
  private currentPing = 0
  private currentSpeedMbps = 0

  private onMessageCb: ((msg: PeerMessage) => void) | null = null
  private onFileProgressCb: ((progress: FileTransferProgress) => void) | null = null
  private onDiagnosticsCb: ((diag: LiveMeshDiagnostics) => void) | null = null
  private onStateChangeCb: ((state: 'disconnected' | 'connecting' | 'connected', peerId?: string) => void) | null = null

  // File chunk assembly buffer: fileId -> { chunks, meta }
  private fileBuffers: Map<string, { chunks: Uint8Array[]; meta: any; received: number }> = new Map()

  constructor() {}

  public onMessage(cb: (msg: PeerMessage) => void) {
    this.onMessageCb = cb
  }

  public onFileProgress(cb: (progress: FileTransferProgress) => void) {
    this.onFileProgressCb = cb
  }

  public onDiagnostics(cb: (diag: LiveMeshDiagnostics) => void) {
    this.onDiagnosticsCb = cb
  }

  public onStateChange(cb: (state: 'disconnected' | 'connecting' | 'connected', peerId?: string) => void) {
    this.onStateChangeCb = cb
  }

  /**
   * Initialize local Peer instance with Google STUN broker
   */
  public async init(custom6DigitCode?: string): Promise<string> {
    if (typeof window === 'undefined') return ''

    const { default: PeerClass } = await import('peerjs')

    const randomSuffix = custom6DigitCode || Math.floor(100000 + Math.random() * 900000).toString()
    const targetPeerId = `aura-${randomSuffix}`

    return new Promise((resolve, reject) => {
      try {
        const peer = new PeerClass(targetPeerId, {
          debug: 0,
          config: {
            iceServers: [
              { urls: 'stun:stun.l.google.com:19302' },
              { urls: 'stun:stun1.l.google.com:19302' },
              { urls: 'stun:stun2.l.google.com:19302' },
            ],
          },
        })

        this.peer = peer

        peer.on('open', (id) => {
          this.myPeerId = id
          const shortCode = id.replace('aura-', '')
          resolve(shortCode)
        })

        peer.on('connection', (conn) => {
          this.handleIncomingConnection(conn)
        })

        peer.on('error', (err) => {
          console.warn('PeerJS event:', err.type)
          if (err.type === 'unavailable-id') {
            const fallbackSuffix = Math.floor(100000 + Math.random() * 900000).toString()
            const fallbackPeer = new PeerClass(`aura-${fallbackSuffix}`)
            this.peer = fallbackPeer
            fallbackPeer.on('open', (id) => {
              this.myPeerId = id
              resolve(id.replace('aura-', ''))
            })
            fallbackPeer.on('connection', (conn) => this.handleIncomingConnection(conn))
          }
        })
      } catch (e) {
        reject(e)
      }
    })
  }

  /**
   * Connect to another peer by their 6-digit code
   */
  public connectToPeer(target6DigitCode: string): Promise<boolean> {
    const fullPeerId = `aura-${target6DigitCode.trim()}`
    if (!this.peer) return Promise.resolve(false)

    if (this.onStateChangeCb) this.onStateChangeCb('connecting', fullPeerId)

    const conn = this.peer.connect(fullPeerId, {
      reliable: true,
    })

    return new Promise((resolve) => {
      conn.on('open', () => {
        this.connection = conn
        this.setupConnectionHandlers(conn)
        if (this.onStateChangeCb) this.onStateChangeCb('connected', fullPeerId)
        this.startHeartbeat()
        resolve(true)
      })

      conn.on('error', () => {
        if (this.onStateChangeCb) this.onStateChangeCb('disconnected')
        resolve(false)
      })
    })
  }

  /**
   * Handle incoming connection when acting as Host
   */
  private handleIncomingConnection(conn: DataConnection) {
    this.connection = conn
    if (this.onStateChangeCb) this.onStateChangeCb('connecting', conn.peer)

    conn.on('open', () => {
      this.setupConnectionHandlers(conn)
      if (this.onStateChangeCb) this.onStateChangeCb('connected', conn.peer)
      this.startHeartbeat()
    })
  }

  /**
   * Setup wire event listeners for real WebRTC DataChannel
   */
  private setupConnectionHandlers(conn: DataConnection) {
    conn.on('data', (data: any) => {
      this.handleIncomingData(data)
    })

    conn.on('close', () => {
      this.cleanupConnection()
    })

    conn.on('error', () => {
      this.cleanupConnection()
    })
  }

  /**
   * Process incoming real packets
   */
  private handleIncomingData(data: any) {
    if (!data) return

    // 1. Text Message
    if (data.type === 'chat') {
      this.bytesReceived += data.text?.length || 0
      if (this.onMessageCb) {
        this.onMessageCb({
          id: data.id || Math.random().toString(36).substring(7),
          sender: 'peer',
          text: data.text,
          timestamp: data.timestamp || Date.now(),
          type: 'chat',
          mediaType: 'text',
        })
      }
      this.emitDiagnostics()
      return
    }

    // 2. Voice Note Message
    if (data.type === 'voice_note') {
      this.bytesReceived += data.audioData?.length || 0
      const audioBlob = new Blob([new Uint8Array(data.audioData)], { type: data.mimeType || 'audio/webm' })
      const audioUrl = URL.createObjectURL(audioBlob)

      if (this.onMessageCb) {
        this.onMessageCb({
          id: data.id || Math.random().toString(36).substring(7),
          sender: 'peer',
          text: '🎤 Voice Message',
          timestamp: data.timestamp || Date.now(),
          type: 'voice_note',
          mediaType: 'audio',
          audioUrl,
        })
      }
      this.emitDiagnostics()
      return
    }

    // 3. Keep-Alive Ping / Pong
    if (data.type === 'ping') {
      this.connection?.send({ type: 'pong', timestamp: data.timestamp })
      return
    }
    if (data.type === 'pong') {
      this.currentPing = Math.max(1, Date.now() - data.timestamp)
      this.emitDiagnostics()
      return
    }

    // 4. File Metadata Header
    if (data.type === 'file_meta') {
      const meta = data.fileMeta
      this.fileBuffers.set(meta.fileId, {
        chunks: [],
        meta,
        received: 0,
      })

      if (this.onFileProgressCb) {
        this.onFileProgressCb({
          id: meta.fileId,
          fileName: meta.fileName,
          fileSize: meta.fileSize,
          fileType: meta.fileType,
          mediaType: meta.mediaType || detectMediaType(meta.fileType, meta.fileName),
          receivedBytes: 0,
          progressPercent: 0,
          isComplete: false,
          sender: 'peer',
        })
      }
      return
    }

    // 5. File Binary Chunk
    if (data.type === 'file_chunk') {
      const { fileId, chunkIdx, chunkBytes } = data
      const entry = this.fileBuffers.get(fileId)
      if (!entry) return

      const bytes = new Uint8Array(chunkBytes)
      entry.chunks[chunkIdx] = bytes
      entry.received += bytes.length
      this.bytesReceived += bytes.length

      const percent = Math.min(100, Math.round((entry.received / entry.meta.fileSize) * 100))
      const isComplete = entry.received >= entry.meta.fileSize || entry.chunks.filter(Boolean).length >= entry.meta.chunksTotal

      let downloadUrl: string | undefined
      let previewUrl: string | undefined

      if (isComplete) {
        const mime = entry.meta.fileType || 'application/octet-stream'
        const blob = new Blob(entry.chunks as any, { type: mime })
        downloadUrl = URL.createObjectURL(blob)
        previewUrl = downloadUrl
      }

      if (this.onFileProgressCb) {
        this.onFileProgressCb({
          id: fileId,
          fileName: entry.meta.fileName,
          fileSize: entry.meta.fileSize,
          fileType: entry.meta.fileType,
          mediaType: entry.meta.mediaType || detectMediaType(entry.meta.fileType, entry.meta.fileName),
          receivedBytes: entry.received,
          progressPercent: percent,
          isComplete,
          downloadUrl,
          previewUrl,
          sender: 'peer',
        })
      }

      this.emitDiagnostics()
      return
    }

    // 6. Bandwidth Burst Chunk
    if (data.type === 'speed_chunk') {
      this.bytesReceived += data.size || 0
      this.emitDiagnostics()
    }
  }

  /**
   * Send real chat message
   */
  public sendChatMessage(text: string) {
    if (!this.connection || !text.trim()) return

    const msg = {
      type: 'chat',
      id: Math.random().toString(36).substring(7),
      text: text.trim(),
      timestamp: Date.now(),
    }

    this.connection.send(msg)
    this.bytesSent += text.length

    if (this.onMessageCb) {
      this.onMessageCb({
        id: msg.id,
        sender: 'self',
        text: msg.text,
        timestamp: msg.timestamp,
        type: 'chat',
        mediaType: 'text',
      })
    }
    this.emitDiagnostics()
  }

  /**
   * Send Voice Note
   */
  public async sendVoiceNote(audioBlob: Blob) {
    if (!this.connection) return

    const arrayBuffer = await audioBlob.arrayBuffer()
    const audioData = Array.from(new Uint8Array(arrayBuffer))
    const msgId = Math.random().toString(36).substring(7)
    const audioUrl = URL.createObjectURL(audioBlob)

    this.connection.send({
      type: 'voice_note',
      id: msgId,
      mimeType: audioBlob.type,
      audioData,
      timestamp: Date.now(),
    })

    this.bytesSent += audioData.length

    if (this.onMessageCb) {
      this.onMessageCb({
        id: msgId,
        sender: 'self',
        text: '🎤 Voice Message',
        timestamp: Date.now(),
        type: 'voice_note',
        mediaType: 'audio',
        audioUrl,
      })
    }
    this.emitDiagnostics()
  }

  /**
   * Send real file in 16KB binary chunks with rich media metadata
   */
  public async sendRealFile(file: File) {
    if (!this.connection) return

    const CHUNK_SIZE = 16 * 1024 // 16 KB per WebRTC frame
    const totalChunks = Math.ceil(file.size / CHUNK_SIZE)
    const fileId = Math.random().toString(36).substring(7)
    const mediaType = detectMediaType(file.type, file.name)
    const localPreviewUrl = URL.createObjectURL(file)

    // Send metadata
    this.connection.send({
      type: 'file_meta',
      fileMeta: {
        fileId,
        fileName: file.name,
        fileSize: file.size,
        fileType: file.type || 'application/octet-stream',
        mediaType,
        chunksTotal: totalChunks,
      },
    })

    if (this.onFileProgressCb) {
      this.onFileProgressCb({
        id: fileId,
        fileName: file.name,
        fileSize: file.size,
        fileType: file.type,
        mediaType,
        receivedBytes: 0,
        progressPercent: 0,
        isComplete: false,
        previewUrl: localPreviewUrl,
        downloadUrl: localPreviewUrl,
        sender: 'self',
      })
    }

    const arrayBuffer = await file.arrayBuffer()
    let offset = 0
    let chunkIdx = 0

    while (offset < file.size) {
      const slice = arrayBuffer.slice(offset, offset + CHUNK_SIZE)
      const chunkBytes = Array.from(new Uint8Array(slice))

      this.connection.send({
        type: 'file_chunk',
        fileId,
        chunkIdx,
        chunkBytes,
      })

      offset += CHUNK_SIZE
      chunkIdx++
      this.bytesSent += slice.byteLength

      const percent = Math.min(100, Math.round((Math.min(offset, file.size) / file.size) * 100))
      if (this.onFileProgressCb) {
        this.onFileProgressCb({
          id: fileId,
          fileName: file.name,
          fileSize: file.size,
          fileType: file.type,
          mediaType,
          receivedBytes: Math.min(offset, file.size),
          progressPercent: percent,
          isComplete: offset >= file.size,
          previewUrl: localPreviewUrl,
          downloadUrl: localPreviewUrl,
          sender: 'self',
        })
      }

      this.emitDiagnostics()
      // Small pause to prevent buffer overflow
      await new Promise((r) => setTimeout(r, 6))
    }
  }

  /**
   * Run real speed burst test between peers
   */
  public async runSpeedBurst() {
    if (!this.connection) return
    const dummyChunk = new Array(8192).fill(0xaa) // 8KB
    const startTime = Date.now()

    for (let i = 0; i < 24; i++) {
      this.connection.send({ type: 'speed_chunk', size: dummyChunk.length, data: dummyChunk })
      this.bytesSent += dummyChunk.length
      await new Promise((r) => setTimeout(r, 4))
    }

    const elapsedSec = Math.max(0.08, (Date.now() - startTime) / 1000)
    const mbps = +(((dummyChunk.length * 24 * 8) / (elapsedSec * 1024 * 1024))).toFixed(2)
    this.currentSpeedMbps = mbps
    this.emitDiagnostics()
  }

  private startHeartbeat() {
    if (this.heartbeatInterval) clearInterval(this.heartbeatInterval)
    this.heartbeatInterval = setInterval(() => {
      if (this.connection) {
        this.lastPingTime = Date.now()
        this.connection.send({ type: 'ping', timestamp: this.lastPingTime })
      }
    }, 2000)
  }

  private emitDiagnostics() {
    if (this.onDiagnosticsCb) {
      this.onDiagnosticsCb({
        pingMs: this.currentPing,
        bytesSent: this.bytesSent,
        bytesReceived: this.bytesReceived,
        realSpeedMbps: this.currentSpeedMbps,
        connectionState: this.connection ? 'connected' : 'disconnected',
        remotePeerId: this.connection?.peer,
      })
    }
  }

  public cleanupConnection() {
    if (this.heartbeatInterval) clearInterval(this.heartbeatInterval)
    if (this.connection) {
      this.connection.close()
      this.connection = null
    }
    if (this.onStateChangeCb) this.onStateChangeCb('disconnected')
    this.emitDiagnostics()
  }

  public disconnect() {
    this.cleanupConnection()
    if (this.peer) {
      this.peer.destroy()
      this.peer = null
    }
  }
}
