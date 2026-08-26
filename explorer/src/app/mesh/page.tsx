'use client'

import React, { useState, useEffect, useRef } from 'react'
import {
  Radio,
  Wifi,
  Shield,
  Zap,
  ArrowUpRight,
  ArrowDownLeft,
  Coins,
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
  Cpu,
  Repeat,
  Power,
  ShieldCheck,
  Gauge,
  AlertTriangle,
  Loader2,
  Sparkles,
} from 'lucide-react'
import {
  RealAuraMeshEngine,
  PeerMessage,
  FileTransferProgress,
  LiveMeshDiagnostics,
} from '@/lib/mesh'

export default function MeshDvpnPage() {
  const [activeTab, setActiveTab] = useState<'pair' | 'chat' | 'files' | 'speed' | 'guide'>('pair')
  const [role, setRole] = useState<'consumer' | 'host'>('consumer')

  // Real WebRTC States
  const [myCode, setMyCode] = useState<string>('')
  const [inputCode, setInputCode] = useState<string>('')
  const [connectionStatus, setConnectionStatus] = useState<'disconnected' | 'connecting' | 'connected'>('disconnected')
  const [connectedPeerCode, setConnectedPeerCode] = useState<string>('')
  const [copiedLink, setCopiedLink] = useState<boolean>(false)
  const [copiedCode, setCopiedCode] = useState<boolean>(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  // Real Chat
  const [messages, setMessages] = useState<PeerMessage[]>([])
  const [chatInput, setChatInput] = useState<string>('')

  // Real Files
  const [fileTransfers, setFileTransfers] = useState<FileTransferProgress[]>([])
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Real Diagnostics
  const [diagnostics, setDiagnostics] = useState<LiveMeshDiagnostics>({
    pingMs: 0,
    bytesSent: 0,
    bytesReceived: 0,
    realSpeedMbps: 0,
    connectionState: 'disconnected',
  })
  const [runningSpeedTest, setRunningSpeedTest] = useState<boolean>(false)

  const engineRef = useRef<RealAuraMeshEngine | null>(null)

  // Initialize Real WebRTC Peer on mount
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

    // Init peer and get real 6-digit code
    engine
      .init()
      .then((code) => {
        setMyCode(code)
      })
      .catch((err) => {
        console.error('WebRTC Init Error:', err)
        setErrorMessage('Failed to initialize WebRTC engine. Please check internet connection.')
      })

    return () => {
      engine.disconnect()
    }
  }, [])

  const handleConnect = async () => {
    const target = inputCode.trim()
    if (!target || target.length < 5) {
      setErrorMessage('Please enter a valid 6-digit pair code.')
      return
    }

    if (target === myCode) {
      setErrorMessage('Cannot connect to your own device code. Open this page on another phone or computer.')
      return
    }

    setErrorMessage(null)
    setConnectionStatus('connecting')

    const success = await engineRef.current?.connectToPeer(target)
    if (!success) {
      setConnectionStatus('disconnected')
      setErrorMessage(`Could not connect to Peer #${target}. Ensure the other device is open on this page.`)
    }
  }

  const handleDisconnect = () => {
    engineRef.current?.cleanupConnection()
    setConnectionStatus('disconnected')
    setConnectedPeerCode('')
    setMessages([])
    setFileTransfers([])
  }

  const handleCopyCode = () => {
    navigator.clipboard.writeText(myCode)
    setCopiedCode(true)
    setTimeout(() => setCopiedCode(false), 2000)
  }

  const handleCopyShareLink = () => {
    if (typeof window !== 'undefined') {
      const shareUrl = `${window.location.origin}/mesh?join=${myCode}`
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

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file || connectionStatus !== 'connected') return

    engineRef.current?.sendRealFile(file)
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
    <div className="space-y-6 max-w-5xl mx-auto font-sans pb-16">
      {/* Mobile-Optimized Top Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border-b border-slate-200 pb-5">
        <div className="flex items-center gap-3">
          <div className="flex h-11 w-11 sm:h-12 sm:w-12 items-center justify-center rounded-2xl bg-gradient-to-tr from-emerald-600 via-teal-500 to-emerald-400 text-white shadow-md shadow-emerald-500/20 shrink-0">
            <Radio className="h-5 w-5 sm:h-6 sm:w-6" />
          </div>
          <div>
            <div className="flex items-center gap-2 flex-wrap">
              <h1 className="text-xl sm:text-2xl font-extrabold text-slate-900">Aura Real P2P Mesh</h1>
              <span
                className={`rounded-full px-2.5 py-0.5 text-[11px] font-bold border font-mono ${
                  isConnected
                    ? 'bg-emerald-50 text-emerald-700 border-emerald-200 animate-pulse'
                    : isConnecting
                    ? 'bg-amber-50 text-amber-700 border-amber-200'
                    : 'bg-slate-100 text-slate-600 border-slate-200'
                }`}
              >
                {isConnected
                  ? `● CONNECTED TO #${connectedPeerCode}`
                  : isConnecting
                  ? '⏳ CONNECTING...'
                  : '○ OFFLINE (DISCONNECTED)'}
              </span>
            </div>
            <p className="text-xs text-slate-500 font-medium mt-0.5">
              100% Real WebRTC Direct DataChannel • Real Binary File Transfer • Live Ping Telemetry
            </p>
          </div>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2 w-full sm:w-auto">
          {isConnected ? (
            <button
              onClick={handleDisconnect}
              className="w-full sm:w-auto flex items-center justify-center gap-1.5 rounded-xl bg-red-50 border border-red-200 px-4 py-2.5 text-xs font-bold text-red-700 hover:bg-red-100 shadow-xs transition-all cursor-pointer"
            >
              <Power className="h-4 w-4" />
              <span>Disconnect Peer</span>
            </button>
          ) : (
            <button
              onClick={() => setActiveTab('pair')}
              className="w-full sm:w-auto flex items-center justify-center gap-1.5 rounded-xl bg-emerald-600 px-4 py-2.5 text-xs font-bold text-white shadow-sm hover:bg-emerald-700 transition-all cursor-pointer"
            >
              <Power className="h-4 w-4" />
              <span>Connect Devices</span>
            </button>
          )}
        </div>
      </div>

      {/* Error Notification */}
      {errorMessage && (
        <div className="rounded-xl border border-red-200 bg-red-50 p-3.5 text-xs text-red-700 flex items-start gap-2.5 font-medium shadow-xs">
          <AlertTriangle className="h-4 w-4 text-red-600 shrink-0 mt-0.5" />
          <div className="flex-1">{errorMessage}</div>
        </div>
      )}

      {/* Mobile-Friendly Segmented Navigation Bar */}
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-1.5 p-1 bg-slate-100/90 rounded-2xl border border-slate-200/80 font-mono text-xs">
        <button
          onClick={() => setActiveTab('pair')}
          className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
            activeTab === 'pair'
              ? 'bg-white text-emerald-700 shadow-sm border border-slate-200/60'
              : 'text-slate-600 hover:text-slate-900'
          }`}
        >
          <Radio className="h-4 w-4" />
          <span>1. Pairing</span>
        </button>

        <button
          onClick={() => setActiveTab('chat')}
          className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
            activeTab === 'chat'
              ? 'bg-white text-emerald-700 shadow-sm border border-slate-200/60'
              : 'text-slate-600 hover:text-slate-900'
          }`}
        >
          <MessageSquare className="h-4 w-4" />
          <span>2. Live Chat</span>
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
              ? 'bg-white text-emerald-700 shadow-sm border border-slate-200/60'
              : 'text-slate-600 hover:text-slate-900'
          }`}
        >
          <FileUp className="h-4 w-4" />
          <span>3. File Share</span>
        </button>

        <button
          onClick={() => setActiveTab('speed')}
          className={`flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
            activeTab === 'speed'
              ? 'bg-white text-emerald-700 shadow-sm border border-slate-200/60'
              : 'text-slate-600 hover:text-slate-900'
          }`}
        >
          <Gauge className="h-4 w-4" />
          <span>4. Speed & Ping</span>
        </button>

        <button
          onClick={() => setActiveTab('guide')}
          className={`col-span-2 sm:col-span-1 flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl font-bold transition-all cursor-pointer ${
            activeTab === 'guide'
              ? 'bg-white text-indigo-700 shadow-sm border border-slate-200/60'
              : 'text-slate-600 hover:text-slate-900'
          }`}
        >
          <Smartphone className="h-4 w-4 text-indigo-600" />
          <span>5. Android VPN Guide</span>
        </button>
      </div>

      {/* TAB 1: REAL PAIRING SECTION */}
      {activeTab === 'pair' && (
        <div className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
            {/* Device A: Your Code Card */}
            <div className="rounded-2xl border border-slate-200/80 bg-white p-5 sm:p-6 shadow-card space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Smartphone className="h-5 w-5 text-emerald-600" />
                  <h2 className="text-sm font-bold text-slate-900">Your Device Code (Share this)</h2>
                </div>
                <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-mono font-bold text-emerald-700 border border-emerald-200">
                  Online
                </span>
              </div>
              <p className="text-xs text-slate-500 leading-relaxed">
                Open this website on another phone or computer, and enter this 6-digit code to connect:
              </p>

              <div className="flex items-center gap-2.5">
                <div className="flex-1 rounded-xl border border-slate-200 bg-slate-50 py-3.5 text-center text-2xl sm:text-3xl font-mono font-extrabold text-emerald-700 tracking-widest select-all shadow-2xs">
                  {myCode || 'Loading...'}
                </div>
                <button
                  onClick={handleCopyCode}
                  className="flex items-center justify-center gap-1 rounded-xl border border-slate-200 bg-white px-3.5 py-3.5 text-xs font-bold text-slate-700 hover:bg-slate-50 cursor-pointer shadow-xs transition-all"
                  title="Copy 6-digit code"
                >
                  {copiedCode ? <Check className="h-4 w-4 text-emerald-600" /> : <Copy className="h-4 w-4" />}
                  <span className="hidden sm:inline">{copiedCode ? 'Copied' : 'Copy'}</span>
                </button>
              </div>

              <button
                onClick={handleCopyShareLink}
                className="w-full flex items-center justify-center gap-1.5 rounded-xl border border-emerald-200 bg-emerald-50/70 py-2.5 text-xs font-bold text-emerald-800 hover:bg-emerald-100/80 transition-colors cursor-pointer"
              >
                <Share2 className="h-3.5 w-3.5" />
                <span>{copiedLink ? 'Share Link Copied to Clipboard!' : 'Copy 1-Click Invite Link'}</span>
              </button>
            </div>

            {/* Device B: Connect to Remote Peer Card */}
            <div className="rounded-2xl border border-slate-200/80 bg-white p-5 sm:p-6 shadow-card space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Globe2 className="h-5 w-5 text-indigo-600" />
                  <h2 className="text-sm font-bold text-slate-900">Connect to Remote Device</h2>
                </div>
                <span className="text-[11px] text-slate-400 font-mono">WebRTC STUN</span>
              </div>
              <p className="text-xs text-slate-500 leading-relaxed">
                Enter the other device's 6-digit code to open a direct peer-to-peer encrypted socket:
              </p>

              <div className="space-y-3">
                <input
                  type="text"
                  maxLength={6}
                  placeholder="Enter 6-digit code"
                  value={inputCode}
                  onChange={(e) => setInputCode(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 bg-slate-50 py-3 px-4 text-center text-xl font-mono font-bold text-slate-900 outline-none focus:border-emerald-500 focus:bg-white focus:ring-4 focus:ring-emerald-500/10 transition-all shadow-2xs"
                />
                <button
                  onClick={handleConnect}
                  disabled={isConnecting || isConnected}
                  className="w-full flex items-center justify-center gap-2 rounded-xl bg-emerald-600 py-3 text-xs font-bold text-white shadow-md shadow-emerald-600/20 hover:bg-emerald-700 cursor-pointer active:scale-98 transition-all disabled:opacity-50"
                >
                  {isConnecting ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      <span>Exchanging ICE Candidates & Connecting...</span>
                    </>
                  ) : isConnected ? (
                    <>
                      <CheckCircle2 className="h-4 w-4" />
                      <span>Connected to #{connectedPeerCode}</span>
                    </>
                  ) : (
                    <>
                      <Power className="h-4 w-4" />
                      <span>Connect Now</span>
                    </>
                  )}
                </button>
              </div>
            </div>
          </div>

          {/* Real-time Hardware Telemetry Bar */}
          <div className="rounded-2xl border border-slate-200/80 bg-white p-5 sm:p-6 shadow-card grid grid-cols-2 sm:grid-cols-4 gap-4 font-mono">
            <div className="border-r border-slate-100 pr-2">
              <span className="text-[11px] text-slate-400 font-sans block">Channel State</span>
              <div className="text-sm sm:text-base font-extrabold text-slate-900 mt-1 truncate">
                {isConnected ? '● ACTIVE' : '○ OFFLINE'}
              </div>
              <span className="text-[10px] text-slate-400 font-sans">
                {isConnected ? `#${connectedPeerCode}` : 'Waiting for connection'}
              </span>
            </div>

            <div className="border-r border-slate-100 pr-2">
              <span className="text-[11px] text-slate-400 font-sans block">Real Bytes Transferred</span>
              <div className="text-sm sm:text-base font-extrabold text-emerald-700 mt-1">
                {totalTransferredMb} <span className="text-xs">MB</span>
              </div>
              <span className="text-[10px] text-slate-400 font-sans">Across DataChannel</span>
            </div>

            <div className="border-r border-slate-100 pr-2">
              <span className="text-[11px] text-slate-400 font-sans block">Direct P2P Ping</span>
              <div className="text-sm sm:text-base font-extrabold text-indigo-700 mt-1">
                {isConnected && diagnostics.pingMs > 0 ? `${diagnostics.pingMs} ms` : '--'}
              </div>
              <span className="text-[10px] text-slate-400 font-sans">Measured Latency</span>
            </div>

            <div>
              <span className="text-[11px] text-slate-400 font-sans block">Security Protocol</span>
              <div className="text-xs sm:text-sm font-bold text-slate-800 mt-1 truncate">
                DTLS / SCTP
              </div>
              <span className="text-[10px] text-slate-400 font-sans">Zero Server Intermediary</span>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: REAL CHAT */}
      {activeTab === 'chat' && (
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 sm:p-6 shadow-card space-y-4">
          <div className="flex items-center justify-between border-b border-slate-100 pb-3">
            <div className="flex items-center gap-2.5">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-50 text-emerald-600 border border-emerald-100">
                <MessageSquare className="h-4 w-4" />
              </div>
              <div>
                <h2 className="text-sm font-bold text-slate-900">Real-Time Encrypted P2P Chat</h2>
                <span className="text-[11px] text-slate-400 font-mono">
                  {isConnected ? `Connected to Peer #${connectedPeerCode}` : 'Offline — Connect Peer to Chat'}
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

          <div className="h-72 sm:h-80 overflow-y-auto space-y-2.5 p-3 sm:p-4 rounded-xl bg-slate-50 border border-slate-200 font-sans">
            {messages.length === 0 ? (
              <div className="h-full flex flex-col items-center justify-center text-slate-400 text-xs font-mono space-y-2">
                <MessageSquare className="h-8 w-8 text-slate-300" />
                <p>{isConnected ? 'No messages yet. Send a message below!' : 'Connect to another device to start chatting directly.'}</p>
              </div>
            ) : (
              messages.map((msg) => (
                <div
                  key={msg.id}
                  className={`flex flex-col ${msg.sender === 'self' ? 'items-end' : 'items-start'}`}
                >
                  <div
                    className={`max-w-[85%] sm:max-w-md rounded-2xl px-3.5 py-2 text-xs shadow-xs break-words ${
                      msg.sender === 'self'
                        ? 'bg-emerald-600 text-white rounded-br-none'
                        : 'bg-white text-slate-900 border border-slate-200 rounded-bl-none'
                    }`}
                  >
                    <p className="leading-relaxed">{msg.text}</p>
                  </div>
                  <span className="text-[10px] text-slate-400 mt-0.5 font-mono px-1">
                    {new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                  </span>
                </div>
              ))
            )}
          </div>

          <form onSubmit={handleSendMessage} className="flex items-center gap-2 pt-1">
            <input
              type="text"
              disabled={!isConnected}
              value={chatInput}
              onChange={(e) => setChatInput(e.target.value)}
              placeholder={isConnected ? 'Type your message...' : 'Connect to a peer first to chat'}
              className="flex-1 rounded-xl border border-slate-200 bg-slate-50 px-3.5 py-2.5 text-xs text-slate-900 outline-none focus:border-emerald-500 focus:bg-white focus:ring-4 focus:ring-emerald-500/10 transition-all font-sans disabled:opacity-50"
            />
            <button
              type="submit"
              disabled={!isConnected || !chatInput.trim()}
              className="flex items-center gap-1.5 rounded-xl bg-emerald-600 px-4 py-2.5 text-xs font-bold text-white shadow-sm hover:bg-emerald-700 cursor-pointer active:scale-98 transition-all font-mono disabled:opacity-50"
            >
              <Send className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">Send</span>
            </button>
          </form>
        </div>
      )}

      {/* TAB 3: REAL FILE TRANSFER */}
      {activeTab === 'files' && (
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 sm:p-6 shadow-card space-y-4">
          <div className="flex items-center justify-between border-b border-slate-100 pb-3">
            <div className="flex items-center gap-2.5">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-50 text-indigo-600 border border-indigo-100">
                <FileUp className="h-4 w-4" />
              </div>
              <div>
                <h2 className="text-sm font-bold text-slate-900">Direct Binary File & Media Transfer</h2>
                <span className="text-[11px] text-slate-400 font-mono">16KB WebRTC Binary Chunks • Zero Server Storage</span>
              </div>
            </div>

            <button
              disabled={!isConnected}
              onClick={() => fileInputRef.current?.click()}
              className="flex items-center gap-1.5 rounded-xl bg-indigo-600 px-3.5 py-2 text-xs font-bold text-white shadow-sm hover:bg-indigo-700 cursor-pointer active:scale-98 transition-all font-mono disabled:opacity-50"
            >
              <Paperclip className="h-3.5 w-3.5" />
              <span>Select File</span>
            </button>
          </div>

          <input
            type="file"
            ref={fileInputRef}
            onChange={handleFileUpload}
            className="hidden"
          />

          <div className="divide-y divide-slate-100">
            {fileTransfers.length === 0 ? (
              <div className="py-12 text-center text-xs text-slate-400 font-mono space-y-2">
                <FileUp className="h-8 w-8 text-slate-300 mx-auto" />
                <p>
                  {isConnected
                    ? 'Click "Select File" to stream photos, videos, audio, or APKs directly to the connected device.'
                    : 'Connect to a peer first before sending files.'}
                </p>
              </div>
            ) : (
              fileTransfers.map((file) => (
                <div key={file.id} className="py-4 space-y-2">
                  <div className="flex items-center justify-between text-xs font-mono">
                    <span className="font-bold text-slate-900 truncate max-w-[200px] sm:max-w-xs">{file.fileName}</span>
                    <span className="text-slate-500">
                      {(file.receivedBytes / (1024 * 1024)).toFixed(2)} MB / {(file.fileSize / (1024 * 1024)).toFixed(2)} MB ({file.progressPercent}%)
                    </span>
                  </div>

                  <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                    <div
                      className="bg-emerald-500 h-2 rounded-full transition-all duration-150"
                      style={{ width: `${file.progressPercent}%` }}
                    />
                  </div>

                  {file.isComplete && (
                    <div className="flex items-center justify-between text-xs text-emerald-700 font-bold pt-1">
                      <span className="flex items-center gap-1">
                        <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600" />
                        {file.sender === 'self' ? 'Sent to Peer' : 'Received & Assembled'}
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
                  )}
                </div>
              ))
            )}
          </div>
        </div>
      )}

      {/* TAB 4: REAL SPEED & PING TEST */}
      {activeTab === 'speed' && (
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 sm:p-6 shadow-card space-y-5">
          <div className="flex items-center justify-between border-b border-slate-100 pb-3">
            <div className="flex items-center gap-2.5">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-50 text-emerald-600 border border-emerald-100">
                <Gauge className="h-4 w-4" />
              </div>
              <div>
                <h2 className="text-sm font-bold text-slate-900">Real P2P Speed & Ping Measurement</h2>
                <span className="text-[11px] text-slate-400 font-mono">Direct Socket Burst Test</span>
              </div>
            </div>

            <button
              disabled={!isConnected || runningSpeedTest}
              onClick={handleRunSpeedTest}
              className="flex items-center gap-1.5 rounded-xl bg-emerald-600 px-4 py-2 text-xs font-bold text-white shadow-sm hover:bg-emerald-700 cursor-pointer active:scale-98 transition-all font-mono disabled:opacity-50"
            >
              {runningSpeedTest ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  <span>Testing Burst...</span>
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
              <span className="text-xs text-slate-500 font-sans">Live Latency (Ping)</span>
              <div className="text-2xl font-extrabold text-indigo-700 mt-1">
                {isConnected && diagnostics.pingMs > 0 ? `${diagnostics.pingMs} ms` : '--'}
              </div>
              <span className="text-[10px] text-slate-400 font-sans">Direct UDP round-trip</span>
            </div>

            <div className="rounded-xl border border-slate-200 bg-slate-50 p-4">
              <span className="text-xs text-slate-500 font-sans">Measured P2P Throughput</span>
              <div className="text-2xl font-extrabold text-emerald-700 mt-1">
                {diagnostics.realSpeedMbps > 0 ? `${diagnostics.realSpeedMbps} Mbps` : '--'}
              </div>
              <span className="text-[10px] text-slate-400 font-sans">Socket burst speed</span>
            </div>

            <div className="rounded-xl border border-slate-200 bg-slate-50 p-4">
              <span className="text-xs text-slate-500 font-sans">Total Session Data</span>
              <div className="text-2xl font-extrabold text-slate-900 mt-1">
                {totalTransferredMb} <span className="text-xs">MB</span>
              </div>
              <span className="text-[10px] text-slate-400 font-sans">Sent: {(diagnostics.bytesSent / 1024).toFixed(1)} KB | Recv: {(diagnostics.bytesReceived / 1024).toFixed(1)} KB</span>
            </div>
          </div>
        </div>
      )}

      {/* TAB 5: HOW TO ROUTE ENTIRE PHONE (THE ANDROID APK & SOCKS5 REALITY GUIDE) */}
      {activeTab === 'guide' && (
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 sm:p-6 shadow-card space-y-4 font-sans text-xs">
          <div className="flex items-center gap-2.5 border-b border-slate-100 pb-3">
            <Smartphone className="h-5 w-5 text-indigo-600" />
            <div>
              <h2 className="text-sm font-bold text-slate-900">How to Route 100% of Android OS Traffic (YouTube / Facebook / Apps)</h2>
              <span className="text-[11px] text-slate-400">Browser Sandbox vs Native Android VpnService</span>
            </div>
          </div>

          <div className="space-y-3 leading-relaxed text-slate-700">
            <div className="rounded-xl border border-amber-200 bg-amber-50/70 p-4 text-amber-900">
              <span className="font-bold block mb-1">⚠️ Important Architectural Truth (Why browser alone cannot route YouTube App):</span>
              <p>
                A web browser tab in Chrome or Safari runs inside a strict <strong>Security Sandbox</strong>. A website tab can directly chat, share files, and stream data over WebRTC, but the phone's operating system does not allow a website to intercept other native apps (like YouTube app or Instagram app).
              </p>
            </div>

            <div className="rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-2">
              <span className="font-bold text-slate-900 block text-sm">To route 100% of your phone's background internet:</span>
              <ol className="list-decimal pl-4 space-y-1.5 text-slate-600">
                <li>
                  <strong>Android APK (Native VpnService):</strong> Run the native Android companion app (`AuraVpnService.kt`) which creates a virtual network interface (<code className="bg-white px-1 py-0.5 rounded border border-slate-200 font-mono text-emerald-700 font-bold">tun0</code>) and routes <code className="bg-white px-1 py-0.5 rounded border border-slate-200 font-mono text-emerald-700 font-bold">0.0.0.0/0</code> through your friend's 5G node.
                </li>
                <li>
                  <strong>Laptop / PC SOCKS5 Proxy:</strong> Set your system proxy or Chrome proxy extension to <code className="bg-white px-1 py-0.5 rounded border border-slate-200 font-mono text-emerald-700 font-bold">SOCKS5 127.0.0.1:1080</code> (served by `aura-tunnel`).
                </li>
                <li>
                  <strong>Browser Web Portal:</strong> Use this web page directly for zero-install <strong>Instant P2P Encrypted Chat</strong> and <strong>High-Speed Binary File/Media Sharing</strong> across any two devices on Earth!
                </li>
              </ol>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
