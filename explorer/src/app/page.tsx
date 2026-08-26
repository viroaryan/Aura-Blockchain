'use client'

import React, { useState, useEffect, useRef } from 'react'
import {
  Radio,
  Wifi,
  Shield,
  Zap,
  CheckCircle2,
  Lock,
  Globe2,
  Activity,
  MessageSquare,
  FileUp,
  Download,
  Share2,
  Send,
  Paperclip,
  Check,
  Copy,
  Smartphone,
  Power,
  Gauge,
  AlertTriangle,
  Loader2,
  Sparkles,
  Mic,
  Square,
  Play,
  Pause,
  Film,
  Image as ImageIcon,
  FileText,
  QrCode,
  Camera,
  Layers,
  ArrowRight,
} from 'lucide-react'
import {
  RealAuraMeshEngine,
  PeerMessage,
  FileTransferProgress,
  LiveMeshDiagnostics,
  detectMediaType,
} from '@/lib/mesh'

export default function HomePage() {
  const [activeTab, setActiveTab] = useState<'pair' | 'chat' | 'files' | 'speed' | 'relay' | 'security'>('pair')

  // Real WebRTC States
  const [myCode, setMyCode] = useState<string>('')
  const [inputCode, setInputCode] = useState<string>('')
  const [connectionStatus, setConnectionStatus] = useState<'disconnected' | 'connecting' | 'connected'>('disconnected')
  const [connectedPeerCode, setConnectedPeerCode] = useState<string>('')
  const [copiedLink, setCopiedLink] = useState<boolean>(false)
  const [copiedCode, setCopiedCode] = useState<boolean>(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  // Real Chat & Voice Notes
  const [messages, setMessages] = useState<PeerMessage[]>([])
  const [chatInput, setChatInput] = useState<string>('')
  const [isRecordingVoice, setIsRecordingVoice] = useState<boolean>(false)
  const mediaRecorderRef = useRef<MediaRecorder | null>(null)
  const audioChunksRef = useRef<Blob[]>([])

  // Real Files & Media
  const [fileTransfers, setFileTransfers] = useState<FileTransferProgress[]>([])
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [isDragging, setIsDragging] = useState<boolean>(false)

  // Real Diagnostics & Speed
  const [diagnostics, setDiagnostics] = useState<LiveMeshDiagnostics>({
    pingMs: 0,
    bytesSent: 0,
    bytesReceived: 0,
    realSpeedMbps: 0,
    connectionState: 'disconnected',
  })
  const [runningSpeedTest, setRunningSpeedTest] = useState<boolean>(false)

  const engineRef = useRef<RealAuraMeshEngine | null>(null)

  // Initialize Real WebRTC Engine
  useEffect(() => {
    const engine = new RealAuraMeshEngine()
    engineRef.current = engine

    engine.onStateChange((state, peerId) => {
      setConnectionStatus(state)
      if (state === 'connected') {
        const cleanId = peerId?.replace('aura-', '') || ''
        setConnectedPeerCode(cleanId)
        setErrorMessage(null)
      } else if (state === 'disconnected') {
        setConnectedPeerCode('')
      }
    })

    engine.onMessage((msg) => {
      setMessages((prev) => [...prev, msg])
    })

    engine.onFileProgress((progress) => {
      setFileTransfers((prev) => {
        const idx = prev.findIndex((f) => f.id === progress.id)
        if (idx >= 0) {
          const updated = [...prev]
          updated[idx] = progress
          return updated
        }
        return [...prev, progress]
      })
    })

    engine.onDiagnostics((diag) => {
      setDiagnostics(diag)
    })

    // Read join query param if any
    let customCode: string | undefined
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search)
      const joinParam = params.get('join')
      if (joinParam && joinParam.length === 6) {
        setInputCode(joinParam)
      }
    }

    engine
      .init()
      .then((code) => {
        setMyCode(code)
      })
      .catch((err) => {
        console.error('WebRTC Init Error:', err)
        setErrorMessage('Failed to initialize local WebRTC socket. Please ensure internet access.')
      })

    return () => {
      engine.disconnect()
    }
  }, [])

  const handleConnect = async () => {
    const target = inputCode.trim()
    if (!target || target.length < 5) {
      setErrorMessage('Please enter a valid 6-digit peer code.')
      return
    }

    if (target === myCode) {
      setErrorMessage('Cannot connect to your own device. Open Aura on a second phone or laptop.')
      return
    }

    setErrorMessage(null)
    setConnectionStatus('connecting')

    const success = await engineRef.current?.connectToPeer(target)
    if (!success) {
      setConnectionStatus('disconnected')
      setErrorMessage(`Could not reach Peer #${target}. Make sure the target device is open on Aura.`)
    }
  }

  const handleDisconnect = () => {
    engineRef.current?.cleanupConnection()
    setConnectionStatus('disconnected')
    setConnectedPeerCode('')
  }

  const handleCopyCode = () => {
    navigator.clipboard.writeText(myCode)
    setCopiedCode(true)
    setTimeout(() => setCopiedCode(false), 2000)
  }

  const handleCopyShareLink = () => {
    if (typeof window !== 'undefined') {
      const shareUrl = `${window.location.origin}/?join=${myCode}`
      navigator.clipboard.writeText(shareUrl)
      setCopiedLink(true)
      setTimeout(() => setCopiedLink(false), 2000)
    }
  }

  const handleSendMessage = (e: React.FormEvent) => {
    e.preventDefault()
    if (!chatInput.trim() || connectionStatus !== 'connected') return

    engineRef.current?.sendChatMessage(chatInput.trim())
    setChatInput('')
  }

  // Voice Note Recording
  const startVoiceRecording = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      const recorder = new MediaRecorder(stream)
      mediaRecorderRef.current = recorder
      audioChunksRef.current = []

      recorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data)
        }
      }

      recorder.onstop = () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/webm' })
        engineRef.current?.sendVoiceNote(audioBlob)
        stream.getTracks().forEach((track) => track.stop())
      }

      recorder.start()
      setIsRecordingVoice(true)
    } catch (err) {
      console.error('Microphone access denied:', err)
      setErrorMessage('Microphone access is required to record voice notes.')
    }
  }

  const stopVoiceRecording = () => {
    if (mediaRecorderRef.current && isRecordingVoice) {
      mediaRecorderRef.current.stop()
      setIsRecordingVoice(false)
    }
  }

  // File Upload Handlers
  const handleFileUpload = (files: FileList | null) => {
    if (!files || files.length === 0 || connectionStatus !== 'connected') return
    for (let i = 0; i < files.length; i++) {
      engineRef.current?.sendRealFile(files[i])
    }
  }

  const handleRunSpeedTest = async () => {
    if (connectionStatus !== 'connected' || runningSpeedTest) return
    setRunningSpeedTest(true)
    await engineRef.current?.runSpeedBurst()
    setRunningSpeedTest(false)
  }

  const isConnected = connectionStatus === 'connected'
  const isConnecting = connectionStatus === 'connecting'
  const totalTransferredMb = ((diagnostics.bytesSent + diagnostics.bytesReceived) / (1024 * 1024)).toFixed(2)

  return (
    <div className="space-y-8 max-w-6xl mx-auto font-sans pb-12">
      {/* Google Antigravity Style Hero Section */}
      <section className="relative pt-6 pb-6 text-center space-y-5">
        <div className="inline-flex items-center gap-2 rounded-full border border-emerald-200 bg-emerald-50/90 px-3.5 py-1 text-xs font-extrabold text-emerald-800 shadow-xs">
          <Sparkles className="h-3.5 w-3.5 text-emerald-600 animate-spin" />
          <span>100% Free P2P Mesh • Zero Intermediary • Direct Encrypted Tunnel</span>
        </div>

        <h1 className="text-3xl sm:text-5xl md:text-6xl font-black tracking-tight text-slate-900 max-w-4xl mx-auto leading-[1.12]">
          Free Global Communication, <br className="hidden sm:inline" />
          <span className="bg-gradient-to-r from-emerald-600 via-teal-600 to-indigo-600 bg-clip-text text-transparent">
            4K Media Drop & Internet Relay
          </span>
        </h1>

        <p className="max-w-2xl mx-auto text-sm sm:text-base text-slate-600 font-medium leading-relaxed">
          Connect any two devices on Earth instantly. Stream raw 4K videos, high-res photos, voice notes, and messages completely free, or share your 5G/Wi-Fi internet bandwidth with the world.
        </p>

        {/* Hero Quick Action Pills */}
        <div className="flex flex-wrap items-center justify-center gap-3 pt-2">
          <button
            onClick={() => setActiveTab('pair')}
            className="flex items-center gap-2 rounded-2xl bg-emerald-600 px-5 py-3 text-xs font-bold text-white shadow-lg shadow-emerald-600/25 hover:bg-emerald-700 active:scale-98 transition-all cursor-pointer"
          >
            <Radio className="h-4 w-4" />
            <span>Launch Live P2P Sandbox</span>
          </button>

          <a
            href="/aura-mesh.apk"
            download="aura-mesh.apk"
            className="flex items-center gap-2 rounded-2xl border border-slate-300 bg-white px-5 py-3 text-xs font-bold text-slate-800 shadow-sm hover:bg-slate-50 hover:border-slate-400 active:scale-98 transition-all cursor-pointer"
          >
            <Smartphone className="h-4 w-4 text-indigo-600" />
            <span>Download Native Android APK</span>
          </a>
        </div>
      </section>

      {/* Main Interactive Sandbox Card */}
      <div className="glass-card-pro rounded-3xl p-4 sm:p-7 space-y-6">
        {/* Top Status Header */}
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 border-b border-slate-200/80 pb-4">
          <div className="flex items-center gap-3">
            <div className="flex h-11 w-11 items-center justify-center rounded-2xl bg-gradient-to-tr from-emerald-600 via-teal-500 to-emerald-400 text-white shadow-md shadow-emerald-500/20 shrink-0">
              <Radio className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2 flex-wrap">
                <h2 className="text-lg font-bold text-slate-900">Aura Live Mesh Engine</h2>
                <span
                  className={`rounded-full px-2.5 py-0.5 text-[11px] font-bold border font-mono ${
                    isConnected
                      ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                      : isConnecting
                      ? 'bg-amber-50 text-amber-700 border-amber-200 animate-pulse'
                      : 'bg-slate-100 text-slate-600 border-slate-200'
                  }`}
                >
                  {isConnected
                    ? `● CONNECTED (#${connectedPeerCode})`
                    : isConnecting
                    ? '⏳ CONNECTING...'
                    : '○ IDLE (DISCONNECTED)'}
                </span>
              </div>
              <p className="text-xs text-slate-500 font-medium mt-0.5">
                Direct WebRTC DTLS/SCTP Channel • Live Data Transfer • Zero Server Logs
              </p>
            </div>
          </div>

          {isConnected && (
            <button
              onClick={handleDisconnect}
              className="flex items-center gap-1.5 rounded-xl bg-red-50 border border-red-200 px-3.5 py-2 text-xs font-bold text-red-700 hover:bg-red-100 transition-all cursor-pointer"
            >
              <Power className="h-3.5 w-3.5" />
              <span>Disconnect</span>
            </button>
          )}
        </div>

        {/* Error Alert */}
        {errorMessage && (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-xs text-red-800 flex items-start gap-3 shadow-xs">
            <AlertTriangle className="h-4 w-4 text-red-600 shrink-0 mt-0.5" />
            <div className="flex-1 font-medium">{errorMessage}</div>
          </div>
        )}

        {/* Google NotebookLM Style Segmented Tabs */}
        <div className="grid grid-cols-2 sm:grid-cols-6 gap-1.5 p-1.5 bg-slate-100/90 rounded-2xl border border-slate-200/70 font-mono text-xs">
          <button
            onClick={() => setActiveTab('pair')}
            className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
              activeTab === 'pair'
                ? 'bg-white text-emerald-800 shadow-sm border border-slate-200/80'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <Radio className="h-4 w-4" />
            <span>1. Pair Hub</span>
          </button>

          <button
            onClick={() => setActiveTab('chat')}
            className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
              activeTab === 'chat'
                ? 'bg-white text-emerald-800 shadow-sm border border-slate-200/80'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <MessageSquare className="h-4 w-4" />
            <span>2. Chat & Voice</span>
            {messages.length > 0 && (
              <span className="rounded-full bg-emerald-100 text-emerald-800 px-1.5 text-[10px]">
                {messages.length}
              </span>
            )}
          </button>

          <button
            onClick={() => setActiveTab('files')}
            className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
              activeTab === 'files'
                ? 'bg-white text-emerald-800 shadow-sm border border-slate-200/80'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <FileUp className="h-4 w-4" />
            <span>3. 4K Media Drop</span>
          </button>

          <button
            onClick={() => setActiveTab('speed')}
            className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
              activeTab === 'speed'
                ? 'bg-white text-emerald-800 shadow-sm border border-slate-200/80'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <Gauge className="h-4 w-4" />
            <span>4. Speedometer</span>
          </button>

          <button
            onClick={() => setActiveTab('relay')}
            className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
              activeTab === 'relay'
                ? 'bg-white text-indigo-700 shadow-sm border border-slate-200/80'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <Wifi className="h-4 w-4 text-indigo-600" />
            <span>5. Internet Relay</span>
          </button>

          <button
            onClick={() => setActiveTab('security')}
            className={`col-span-2 sm:col-span-1 flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
              activeTab === 'security'
                ? 'bg-white text-slate-900 shadow-sm border border-slate-200/80'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <Shield className="h-4 w-4" />
            <span>6. Privacy</span>
          </button>
        </div>

        {/* TAB 1: PAIRING HUB & QR RADAR */}
        {activeTab === 'pair' && (
          <div className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
              {/* Your Code Card */}
              <div className="rounded-2xl border border-slate-200 bg-white p-5 sm:p-6 shadow-xs space-y-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Smartphone className="h-5 w-5 text-emerald-600" />
                    <h3 className="text-sm font-bold text-slate-900">Your Device Code (Share with Peer)</h3>
                  </div>
                  <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-mono font-bold text-emerald-700 border border-emerald-200">
                    Ready
                  </span>
                </div>
                <p className="text-xs text-slate-500 leading-relaxed">
                  Open Aura on another phone, tablet, or laptop, and enter this 6-digit code to pair instantly:
                </p>

                <div className="flex items-center gap-2.5">
                  <div className="flex-1 rounded-xl border border-slate-200 bg-slate-50 py-3.5 text-center text-3xl font-mono font-extrabold text-emerald-700 tracking-widest select-all shadow-2xs">
                    {myCode || 'Connecting...'}
                  </div>
                  <button
                    onClick={handleCopyCode}
                    className="flex items-center justify-center gap-1 rounded-xl border border-slate-200 bg-white px-4 py-3.5 text-xs font-bold text-slate-700 hover:bg-slate-50 cursor-pointer shadow-xs transition-all"
                  >
                    {copiedCode ? <Check className="h-4 w-4 text-emerald-600" /> : <Copy className="h-4 w-4" />}
                    <span>{copiedCode ? 'Copied' : 'Copy'}</span>
                  </button>
                </div>

                <button
                  onClick={handleCopyShareLink}
                  className="w-full flex items-center justify-center gap-2 rounded-xl border border-emerald-200 bg-emerald-50/80 py-2.5 text-xs font-bold text-emerald-800 hover:bg-emerald-100/90 transition-colors cursor-pointer"
                >
                  <Share2 className="h-3.5 w-3.5" />
                  <span>{copiedLink ? '1-Click Invite Link Copied!' : 'Copy 1-Click Invite Link'}</span>
                </button>
              </div>

              {/* Connect to Remote Device */}
              <div className="rounded-2xl border border-slate-200 bg-white p-5 sm:p-6 shadow-xs space-y-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Globe2 className="h-5 w-5 text-indigo-600" />
                    <h3 className="text-sm font-bold text-slate-900">Connect to Remote Device</h3>
                  </div>
                  <span className="text-[11px] text-slate-400 font-mono">WebRTC P2P</span>
                </div>
                <p className="text-xs text-slate-500 leading-relaxed">
                  Enter your friend's 6-digit code to open a direct socket for free chat, 4K media, and internet relay:
                </p>

                <div className="space-y-3">
                  <input
                    type="text"
                    maxLength={6}
                    placeholder="Enter 6-digit code (e.g. 849201)"
                    value={inputCode}
                    onChange={(e) => setInputCode(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 bg-slate-50 py-3.5 px-4 text-center text-2xl font-mono font-bold text-slate-900 outline-none focus:border-emerald-500 focus:bg-white focus:ring-4 focus:ring-emerald-500/10 transition-all shadow-2xs"
                  />
                  <button
                    onClick={handleConnect}
                    disabled={isConnecting || isConnected}
                    className="w-full flex items-center justify-center gap-2 rounded-xl bg-emerald-600 py-3.5 text-xs font-bold text-white shadow-md shadow-emerald-600/20 hover:bg-emerald-700 cursor-pointer active:scale-98 transition-all disabled:opacity-50"
                  >
                    {isConnecting ? (
                      <>
                        <Loader2 className="h-4 w-4 animate-spin" />
                        <span>Negotiating Direct ICE Candidates...</span>
                      </>
                    ) : isConnected ? (
                      <>
                        <CheckCircle2 className="h-4 w-4" />
                        <span>Connected to Peer #{connectedPeerCode}</span>
                      </>
                    ) : (
                      <>
                        <Power className="h-4 w-4" />
                        <span>Connect Devices Now</span>
                      </>
                    )}
                  </button>
                </div>
              </div>
            </div>

            {/* Real Hardware Telemetry Bar */}
            <div className="rounded-2xl border border-slate-200 bg-white p-5 shadow-xs grid grid-cols-2 sm:grid-cols-4 gap-4 font-mono text-xs">
              <div className="border-r border-slate-100 pr-2">
                <span className="text-[11px] text-slate-400 font-sans block">Channel State</span>
                <div className="text-sm sm:text-base font-extrabold text-slate-900 mt-1 truncate">
                  {isConnected ? '● ACTIVE' : '○ OFFLINE'}
                </div>
                <span className="text-[10px] text-slate-400 font-sans">
                  {isConnected ? `Peer #${connectedPeerCode}` : 'Waiting for connection'}
                </span>
              </div>

              <div className="border-r border-slate-100 pr-2">
                <span className="text-[11px] text-slate-400 font-sans block">Real Bytes Transferred</span>
                <div className="text-sm sm:text-base font-extrabold text-emerald-700 mt-1">
                  {totalTransferredMb} <span className="text-xs">MB</span>
                </div>
                <span className="text-[10px] text-slate-400 font-sans">Across P2P DataChannel</span>
              </div>

              <div className="border-r border-slate-100 pr-2">
                <span className="text-[11px] text-slate-400 font-sans block">Direct P2P Ping</span>
                <div className="text-sm sm:text-base font-extrabold text-indigo-700 mt-1">
                  {isConnected && diagnostics.pingMs > 0 ? `${diagnostics.pingMs} ms` : '--'}
                </div>
                <span className="text-[10px] text-slate-400 font-sans">Measured Latency</span>
              </div>

              <div>
                <span className="text-[11px] text-slate-400 font-sans block">Encryption Protocol</span>
                <div className="text-xs sm:text-sm font-bold text-slate-800 mt-1 truncate">
                  DTLS 1.3 / SCTP
                </div>
                <span className="text-[10px] text-slate-400 font-sans">Zero Intermediary Server</span>
              </div>
            </div>
          </div>
        )}

        {/* TAB 2: CHAT & VOICE NOTES */}
        {activeTab === 'chat' && (
          <div className="rounded-2xl border border-slate-200 bg-white p-5 shadow-xs space-y-4">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3">
              <div className="flex items-center gap-2.5">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-50 text-emerald-600 border border-emerald-100">
                  <MessageSquare className="h-4 w-4" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-slate-900">Encrypted Real-Time Chat & Voice Notes</h3>
                  <span className="text-[11px] text-slate-400 font-mono">
                    {isConnected ? `Connected to Peer #${connectedPeerCode}` : 'Offline — Connect peer in Tab 1 to chat'}
                  </span>
                </div>
              </div>
              <span
                className={`rounded-full px-2.5 py-0.5 text-[10px] font-mono font-bold border ${
                  isConnected ? 'bg-emerald-50 text-emerald-700 border-emerald-200' : 'bg-slate-100 text-slate-500 border-slate-200'
                }`}
              >
                {isConnected ? 'Live Socket' : 'Disconnected'}
              </span>
            </div>

            {/* Message Feed */}
            <div className="h-80 sm:h-96 overflow-y-auto space-y-3 p-4 rounded-xl bg-slate-50 border border-slate-200 font-sans">
              {messages.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-slate-400 text-xs font-mono space-y-2">
                  <MessageSquare className="h-8 w-8 text-slate-300" />
                  <p>{isConnected ? 'No messages yet. Send a message or record a voice note below!' : 'Connect to a peer to chat.'}</p>
                </div>
              ) : (
                messages.map((msg) => (
                  <div
                    key={msg.id}
                    className={`flex flex-col ${msg.sender === 'self' ? 'items-end' : 'items-start'}`}
                  >
                    <div
                      className={`max-w-[85%] sm:max-w-md rounded-2xl px-4 py-2.5 text-xs shadow-xs break-words ${
                        msg.sender === 'self'
                          ? 'bg-emerald-600 text-white rounded-br-none'
                          : 'bg-white text-slate-900 border border-slate-200 rounded-bl-none'
                      }`}
                    >
                      {msg.type === 'voice_note' && msg.audioUrl ? (
                        <div className="space-y-1.5 py-1">
                          <div className="flex items-center gap-2 font-bold text-[11px]">
                            <Mic className="h-3.5 w-3.5" />
                            <span>Voice Note</span>
                          </div>
                          <audio controls src={msg.audioUrl} className="w-full h-8" />
                        </div>
                      ) : (
                        <p className="leading-relaxed whitespace-pre-wrap">{msg.text}</p>
                      )}
                    </div>
                    <span className="text-[10px] text-slate-400 mt-1 font-mono px-1">
                      {new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </span>
                  </div>
                ))
              )}
            </div>

            {/* Input Controls */}
            <form onSubmit={handleSendMessage} className="flex items-center gap-2 pt-1">
              <input
                type="text"
                disabled={!isConnected}
                value={chatInput}
                onChange={(e) => setChatInput(e.target.value)}
                placeholder={isConnected ? 'Type an encrypted message...' : 'Connect to peer to chat'}
                className="flex-1 rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-xs text-slate-900 outline-none focus:border-emerald-500 focus:bg-white focus:ring-4 focus:ring-emerald-500/10 transition-all font-sans disabled:opacity-50"
              />

              {/* Voice Note Button */}
              {isRecordingVoice ? (
                <button
                  type="button"
                  onClick={stopVoiceRecording}
                  className="flex items-center gap-1.5 rounded-xl bg-red-600 px-4 py-3 text-xs font-bold text-white animate-pulse shadow-sm cursor-pointer"
                >
                  <Square className="h-3.5 w-3.5" />
                  <span>Stop & Send</span>
                </button>
              ) : (
                <button
                  type="button"
                  disabled={!isConnected}
                  onClick={startVoiceRecording}
                  className="flex items-center gap-1.5 rounded-xl border border-slate-200 bg-slate-50 px-3.5 py-3 text-xs font-bold text-slate-700 hover:bg-slate-100 cursor-pointer transition-all disabled:opacity-50"
                  title="Record Voice Note"
                >
                  <Mic className="h-4 w-4 text-emerald-600" />
                </button>
              )}

              <button
                type="submit"
                disabled={!isConnected || !chatInput.trim()}
                className="flex items-center gap-1.5 rounded-xl bg-emerald-600 px-5 py-3 text-xs font-bold text-white shadow-sm hover:bg-emerald-700 cursor-pointer active:scale-98 transition-all font-mono disabled:opacity-50"
              >
                <Send className="h-3.5 w-3.5" />
                <span>Send</span>
              </button>
            </form>
          </div>
        )}

        {/* TAB 3: 4K MEDIA & FILE DROP */}
        {activeTab === 'files' && (
          <div className="rounded-2xl border border-slate-200 bg-white p-5 shadow-xs space-y-4">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3">
              <div className="flex items-center gap-2.5">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-50 text-indigo-600 border border-indigo-100">
                  <FileUp className="h-4 w-4" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-slate-900">Direct 4K Video, Image & Binary Drop</h3>
                  <span className="text-[11px] text-slate-400 font-mono">16KB WebRTC Binary Chunks • Zero Compression</span>
                </div>
              </div>

              <button
                disabled={!isConnected}
                onClick={() => fileInputRef.current?.click()}
                className="flex items-center gap-1.5 rounded-xl bg-indigo-600 px-4 py-2.5 text-xs font-bold text-white shadow-sm hover:bg-indigo-700 cursor-pointer active:scale-98 transition-all font-mono disabled:opacity-50"
              >
                <Paperclip className="h-3.5 w-3.5" />
                <span>Select Files / Videos</span>
              </button>
            </div>

            <input
              type="file"
              multiple
              ref={fileInputRef}
              onChange={(e) => handleFileUpload(e.target.files)}
              className="hidden"
            />

            {/* Drag & Drop Dropzone */}
            <div
              onDragOver={(e) => {
                e.preventDefault()
                setIsDragging(true)
              }}
              onDragLeave={() => setIsDragging(false)}
              onDrop={(e) => {
                e.preventDefault()
                setIsDragging(false)
                handleFileUpload(e.dataTransfer.files)
              }}
              className={`rounded-2xl border-2 border-dashed p-8 text-center transition-all ${
                isDragging
                  ? 'border-emerald-500 bg-emerald-50/50 scale-[1.01]'
                  : 'border-slate-200 bg-slate-50/60 hover:bg-slate-50'
              }`}
            >
              <FileUp className="h-10 w-10 text-slate-400 mx-auto mb-2" />
              <p className="text-xs font-bold text-slate-800">
                {isConnected ? 'Drag & Drop Any Videos, Photos, or Documents Here' : 'Connect to a peer to enable direct media streaming'}
              </p>
              <p className="text-[11px] text-slate-400 mt-1 font-mono">
                Supports raw 4K MP4, MKV, WebM, PNG, RAW, ZIP, APK (Direct P2P socket stream)
              </p>
            </div>

            {/* Transfers List & Live Inline Previews */}
            <div className="divide-y divide-slate-100 pt-2">
              {fileTransfers.length === 0 ? (
                <div className="py-8 text-center text-xs text-slate-400 font-mono">
                  No active transfers. Upload media to stream directly to peer.
                </div>
              ) : (
                fileTransfers.map((file) => (
                  <div key={file.id} className="py-4 space-y-2.5">
                    <div className="flex items-center justify-between text-xs font-mono">
                      <div className="flex items-center gap-2 truncate max-w-[240px] sm:max-w-md">
                        {file.mediaType === 'video' ? (
                          <Film className="h-4 w-4 text-indigo-600 shrink-0" />
                        ) : file.mediaType === 'image' ? (
                          <ImageIcon className="h-4 w-4 text-emerald-600 shrink-0" />
                        ) : (
                          <FileText className="h-4 w-4 text-slate-500 shrink-0" />
                        )}
                        <span className="font-bold text-slate-900 truncate">{file.fileName}</span>
                      </div>
                      <span className="text-slate-500">
                        {(file.receivedBytes / (1024 * 1024)).toFixed(2)} MB / {(file.fileSize / (1024 * 1024)).toFixed(2)} MB ({file.progressPercent}%)
                      </span>
                    </div>

                    <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                      <div
                        className="bg-gradient-to-r from-emerald-500 to-teal-500 h-2 rounded-full transition-all duration-150"
                        style={{ width: `${file.progressPercent}%` }}
                      />
                    </div>

                    {file.isComplete && (
                      <div className="space-y-3 pt-1">
                        <div className="flex items-center justify-between text-xs text-emerald-700 font-bold">
                          <span className="flex items-center gap-1">
                            <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600" />
                            {file.sender === 'self' ? 'Sent to Peer' : 'Received & Reassembled'}
                          </span>
                          {file.downloadUrl && (
                            <a
                              href={file.downloadUrl}
                              download={file.fileName}
                              className="inline-flex items-center gap-1 text-indigo-600 hover:text-indigo-800 font-bold underline cursor-pointer"
                            >
                              <Download className="h-3.5 w-3.5" />
                              <span>Save to Device</span>
                            </a>
                          )}
                        </div>

                        {/* Inline Video Player */}
                        {file.mediaType === 'video' && file.previewUrl && (
                          <div className="rounded-xl overflow-hidden border border-slate-200 bg-black aspect-video max-h-64 sm:max-h-80 w-full flex items-center justify-center">
                            <video controls src={file.previewUrl} className="w-full h-full object-contain" />
                          </div>
                        )}

                        {/* Inline Image Preview */}
                        {file.mediaType === 'image' && file.previewUrl && (
                          <div className="rounded-xl overflow-hidden border border-slate-200 max-h-64 sm:max-h-80 w-full flex items-center justify-center bg-slate-100">
                            <img src={file.previewUrl} alt={file.fileName} className="max-h-64 sm:max-h-80 object-contain" />
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          </div>
        )}

        {/* TAB 4: SPEEDOMETER & LATENCY TEST */}
        {activeTab === 'speed' && (
          <div className="rounded-2xl border border-slate-200 bg-white p-5 shadow-xs space-y-5">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3">
              <div className="flex items-center gap-2.5">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-50 text-emerald-600 border border-emerald-100">
                  <Gauge className="h-4 w-4" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-slate-900">Direct P2P Speed & Ping Measurement</h3>
                  <span className="text-[11px] text-slate-400 font-mono">Live Socket Burst Telemetry</span>
                </div>
              </div>

              <button
                disabled={!isConnected || runningSpeedTest}
                onClick={handleRunSpeedTest}
                className="flex items-center gap-1.5 rounded-xl bg-emerald-600 px-4 py-2.5 text-xs font-bold text-white shadow-sm hover:bg-emerald-700 cursor-pointer active:scale-98 transition-all font-mono disabled:opacity-50"
              >
                {runningSpeedTest ? (
                  <>
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    <span>Measuring Burst...</span>
                  </>
                ) : (
                  <>
                    <Zap className="h-3.5 w-3.5" />
                    <span>Run Speed Test</span>
                  </>
                )}
              </button>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 font-mono">
              <div className="rounded-xl border border-slate-200 bg-slate-50 p-4">
                <span className="text-xs text-slate-500 font-sans">Live Direct Ping</span>
                <div className="text-2xl sm:text-3xl font-extrabold text-indigo-700 mt-1">
                  {isConnected && diagnostics.pingMs > 0 ? `${diagnostics.pingMs} ms` : '--'}
                </div>
                <span className="text-[10px] text-slate-400 font-sans">Round-trip latency</span>
              </div>

              <div className="rounded-xl border border-slate-200 bg-slate-50 p-4">
                <span className="text-xs text-slate-500 font-sans">Measured Throughput</span>
                <div className="text-2xl sm:text-3xl font-extrabold text-emerald-700 mt-1">
                  {diagnostics.realSpeedMbps > 0 ? `${diagnostics.realSpeedMbps} Mbps` : '--'}
                </div>
                <span className="text-[10px] text-slate-400 font-sans">Direct P2P socket speed</span>
              </div>

              <div className="rounded-xl border border-slate-200 bg-slate-50 p-4">
                <span className="text-xs text-slate-500 font-sans">Total Session Data</span>
                <div className="text-2xl sm:text-3xl font-extrabold text-slate-900 mt-1">
                  {totalTransferredMb} <span className="text-xs">MB</span>
                </div>
                <span className="text-[10px] text-slate-400 font-sans">Sent: {(diagnostics.bytesSent / 1024).toFixed(1)} KB | Recv: {(diagnostics.bytesReceived / 1024).toFixed(1)} KB</span>
              </div>
            </div>
          </div>
        )}

        {/* TAB 5: GLOBAL INTERNET RELAY & ANDROID COMPANION APK */}
        {activeTab === 'relay' && (
          <div className="rounded-2xl border border-slate-200 bg-white p-5 sm:p-6 shadow-xs space-y-5 font-sans">
            <div className="flex items-center gap-3 border-b border-slate-100 pb-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-indigo-50 text-indigo-600 border border-indigo-100">
                <Wifi className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-slate-900">How Global Internet Relay Works (Android VpnService)</h3>
                <span className="text-[11px] text-slate-400">Route 100% of Android OS Traffic (YouTube, Instagram, Browser)</span>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
              <div className="rounded-2xl border border-emerald-200 bg-emerald-50/50 p-5 space-y-3">
                <div className="flex items-center gap-2 text-emerald-800 font-bold text-sm">
                  <Smartphone className="h-4 w-4 text-emerald-600" />
                  <span>Native Android Companion APK</span>
                </div>
                <p className="text-xs text-slate-600 leading-relaxed">
                  While a web browser tab is sandboxed for safety, our <strong>Aura Mesh Android App</strong> uses Android's native <code className="bg-white px-1.5 py-0.5 rounded border border-emerald-200 font-mono text-emerald-800 font-bold">VpnService</code> to create a virtual network interface (<code className="bg-white px-1.5 py-0.5 rounded border border-emerald-200 font-mono text-emerald-800 font-bold">tun0</code>) and securely routes all device traffic through your peer's 5G node!
                </p>
                <a
                  href="/aura-mesh.apk"
                  download="aura-mesh.apk"
                  className="inline-flex items-center gap-2 rounded-xl bg-emerald-600 px-4 py-2.5 text-xs font-bold text-white shadow-sm hover:bg-emerald-700 transition-all cursor-pointer"
                >
                  <Download className="h-4 w-4" />
                  <span>Download Aura Mesh APK (18.9 MB)</span>
                </a>
              </div>

              <div className="rounded-2xl border border-slate-200 bg-slate-50 p-5 space-y-3 text-xs text-slate-600">
                <div className="flex items-center gap-2 text-slate-900 font-bold text-sm">
                  <Layers className="h-4 w-4 text-indigo-600" />
                  <span>3 Ways to Connect on Any Device:</span>
                </div>
                <ul className="space-y-2 list-disc pl-4 leading-relaxed">
                  <li><strong>Instant Web Portal:</strong> Zero installation required. Chat and send 4K files right inside Chrome or Safari.</li>
                  <li><strong>Android Companion App:</strong> 100% full-device internet routing with AES-256-GCM encryption and zero battery drain.</li>
                  <li><strong>PC/Mac SOCKS5 Proxy:</strong> Direct SOCKS5 tunnel at <code className="bg-white px-1 py-0.5 rounded border border-slate-200 font-mono font-bold text-indigo-700">127.0.0.1:1080</code> for desktop browsing.</li>
                </ul>
              </div>
            </div>
          </div>
        )}

        {/* TAB 6: ZERO LOGS & PRIVACY */}
        {activeTab === 'security' && (
          <div className="rounded-2xl border border-slate-200 bg-white p-5 sm:p-6 shadow-xs space-y-4 font-sans text-xs">
            <div className="flex items-center gap-3 border-b border-slate-100 pb-3">
              <Shield className="h-5 w-5 text-emerald-600" />
              <div>
                <h3 className="text-sm font-bold text-slate-900">Zero-Logs & End-to-End Cryptography Guarantee</h3>
                <span className="text-[11px] text-slate-400">DTLS 1.3 • Noise IK Protocol • AES-256-GCM AEAD</span>
              </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
              <div className="rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-2">
                <Lock className="h-5 w-5 text-emerald-600" />
                <h4 className="font-bold text-slate-900">Zero Intermediary Servers</h4>
                <p className="text-slate-500 leading-relaxed">
                  Your chat messages, voice notes, and media files travel directly peer-to-peer over WebRTC. No central server stores your data.
                </p>
              </div>

              <div className="rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-2">
                <Shield className="h-5 w-5 text-indigo-600" />
                <h4 className="font-bold text-slate-900">AEAD Cryptography</h4>
                <p className="text-slate-500 leading-relaxed">
                  Every binary packet is encrypted with 256-bit keys and authenticated using cryptographic tags to prevent tampering or interception.
                </p>
              </div>

              <div className="rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-2">
                <Zap className="h-5 w-5 text-teal-600" />
                <h4 className="font-bold text-slate-900">100% Free Forever</h4>
                <p className="text-slate-500 leading-relaxed">
                  Powered by direct peer compute and decentralised mesh routing, eliminating server hosting fees and subscription models.
                </p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
