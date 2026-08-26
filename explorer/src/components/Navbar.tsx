'use client'

import React from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { Radio, MessageSquare, FileUp, Gauge, Smartphone, Sparkles, Shield, Wifi } from 'lucide-react'

export function Navbar() {
  const pathname = usePathname()

  const navItems = [
    { href: '/', label: 'P2P Mesh Hub', icon: Radio, highlight: true },
    { href: '/mesh?tab=chat', label: 'Live Chat & Voice', icon: MessageSquare },
    { href: '/mesh?tab=files', label: '4K Media Drop', icon: FileUp },
    { href: '/mesh?tab=speed', label: 'Speed & Ping', icon: Gauge },
    { href: '/mesh?tab=guide', label: 'Android APK', icon: Smartphone },
  ]

  return (
    <>
      {/* Top Header Bar - Google Antigravity Style */}
      <header className="sticky top-0 z-50 border-b border-slate-200/80 bg-white/85 backdrop-blur-2xl shadow-xs transition-all">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3 sm:px-6">
          {/* Brand Logo */}
          <Link href="/" className="flex items-center gap-3 group">
            <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-gradient-to-tr from-emerald-600 via-teal-500 to-emerald-400 text-white shadow-md shadow-emerald-500/25 group-hover:scale-105 transition-all duration-300 shrink-0">
              <Radio className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="text-lg font-black tracking-tight text-slate-900 font-sans">
                  AURA
                </span>
                <span className="rounded-full bg-emerald-50 px-2.5 py-0.5 text-[10px] font-extrabold text-emerald-700 border border-emerald-200/80 font-mono badge-glow">
                  FREE P2P MESH
                </span>
              </div>
              <span className="text-[11px] font-medium text-slate-400 block -mt-0.5">
                Zero Cost • Encrypted • Global Internet Relay
              </span>
            </div>
          </Link>

          {/* Desktop Navigation Links */}
          <nav className="hidden md:flex items-center gap-2">
            {navItems.map((item) => {
              const Icon = item.icon
              const isActive = pathname === item.href

              if (item.highlight) {
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={`flex items-center gap-1.5 px-4 py-2 rounded-full text-xs font-bold transition-all shadow-xs ${
                      isActive
                        ? 'bg-emerald-600 text-white shadow-emerald-600/30'
                        : 'bg-emerald-50 text-emerald-800 hover:bg-emerald-100 border border-emerald-200/80'
                    }`}
                  >
                    <Radio className="h-3.5 w-3.5 animate-pulse" />
                    <span>{item.label}</span>
                  </Link>
                )
              }

              return (
                <Link
                  key={item.href}
                  href={item.href}
                  className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold transition-all ${
                    isActive
                      ? 'bg-slate-100 text-emerald-700 font-bold'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`}
                >
                  <Icon className="h-3.5 w-3.5" />
                  <span>{item.label}</span>
                </Link>
              )
            })}
          </nav>

          {/* Live Network & APK Quick Action */}
          <div className="flex items-center gap-2.5">
            <a
              href="/aura-mesh.apk"
              download="aura-mesh.apk"
              className="hidden sm:inline-flex items-center gap-1.5 rounded-full bg-slate-900 px-3.5 py-1.5 text-xs font-bold text-white hover:bg-slate-800 transition-all shadow-sm cursor-pointer"
            >
              <Smartphone className="h-3.5 w-3.5 text-emerald-400" />
              <span>Get APK</span>
            </a>

            <div className="flex items-center gap-1.5 rounded-full border border-emerald-200 bg-emerald-50/80 px-2.5 py-1 text-[11px] font-mono font-bold text-emerald-800 shadow-2xs">
              <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse" />
              <span>Mesh Live</span>
            </div>
          </div>
        </div>
      </header>

      {/* Mobile Native Navigation Bar */}
      <div className="md:hidden fixed bottom-0 left-0 right-0 z-50 bg-white/95 backdrop-blur-2xl border-t border-slate-200/90 px-2 py-2 shadow-2xl">
        <div className="flex items-center justify-around">
          {navItems.map((item) => {
            const Icon = item.icon
            const isActive = pathname === item.href

            return (
              <Link
                key={item.href}
                href={item.href}
                className={`flex flex-col items-center justify-center py-1 px-2.5 rounded-xl transition-all active:scale-95 ${
                  isActive
                    ? 'text-emerald-700 font-extrabold'
                    : 'text-slate-500 font-medium hover:text-slate-900'
                }`}
              >
                <div
                  className={`p-1 rounded-xl transition-all ${
                    isActive ? 'bg-emerald-50 text-emerald-700 shadow-2xs' : ''
                  }`}
                >
                  <Icon className="h-5 w-5" />
                </div>
                <span className="text-[10px] mt-0.5">{item.label}</span>
              </Link>
            )
          })}
        </div>
      </div>
    </>
  )
}
