'use client'

import React from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { Box, Shield, Wallet, Radio, Coins, Activity, Globe } from 'lucide-react'

export function Navbar({ chainId }: { chainId?: string }) {
  const pathname = usePathname()

  const navItems = [
    { href: '/', label: 'Explorer', icon: Box },
    { href: '/wallet', label: 'Web Wallet', icon: Wallet },
    { href: '/mesh', label: 'Mesh dVPN', icon: Radio, highlight: true },
    { href: '/validators', label: 'Validators', icon: Shield },
  ]

  return (
    <header className="sticky top-0 z-50 border-b border-slate-200/80 bg-white/80 backdrop-blur-xl shadow-xs transition-all">
      <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3.5 sm:px-6">
        {/* Brand Logo */}
        <Link href="/" className="flex items-center gap-3 group">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-tr from-emerald-600 via-teal-500 to-emerald-400 text-white shadow-md shadow-emerald-500/20 group-hover:scale-105 transition-transform duration-200">
            <Coins className="h-5 w-5 font-bold" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="text-lg font-extrabold tracking-tight text-slate-900 font-sans">
                AURA
              </span>
              <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-bold text-emerald-700 border border-emerald-200">
                MAINNET
              </span>
            </div>
            <span className="text-[11px] font-medium text-slate-500">PoS-BFT & dVPN Network</span>
          </div>
        </Link>

        {/* Navigation Links */}
        <nav className="flex items-center gap-1.5 sm:gap-2">
          {navItems.map((item) => {
            const Icon = item.icon
            const isActive = pathname === item.href

            if (item.highlight) {
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  className={`flex items-center gap-1.5 px-3.5 py-1.5 rounded-full text-xs font-bold transition-all shadow-xs ${
                    isActive
                      ? 'bg-emerald-600 text-white shadow-emerald-600/20'
                      : 'bg-emerald-50 text-emerald-700 hover:bg-emerald-100 border border-emerald-200/80'
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
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  isActive
                    ? 'bg-slate-100 text-emerald-700 font-bold'
                    : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                }`}
              >
                <Icon className="h-3.5 w-3.5" />
                <span className="hidden sm:inline">{item.label}</span>
              </Link>
            )
          })}
        </nav>

        {/* Live Network Status Pill */}
        <div className="hidden md:flex items-center gap-2.5">
          <div className="flex items-center gap-2 rounded-full border border-slate-200 bg-slate-50/80 px-3 py-1 text-xs font-medium text-slate-700 shadow-2xs font-mono">
            <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse" />
            <span>RPC: Online</span>
          </div>
        </div>
      </div>
    </header>
  )
}
