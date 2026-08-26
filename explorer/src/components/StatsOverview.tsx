'use client'

import React from 'react'
import { Box, Shield, Layers, Zap, ArrowUpRight } from 'lucide-react'
import { formatHash } from '@/lib/utils'

interface StatsProps {
  height: number
  stateRoot: string
  validatorsCount: number
  mempoolSize: number
  peerCount: number
}

export function StatsOverview({
  height,
  stateRoot,
  validatorsCount,
  mempoolSize,
  peerCount,
}: StatsProps) {
  const stats = [
    {
      title: 'Committed Height',
      value: `#${height.toLocaleString()}`,
      sub: 'PoS-BFT Finalized',
      icon: Box,
      iconBg: 'bg-emerald-50 text-emerald-600 border border-emerald-100',
      badge: 'Active',
      badgeColor: 'bg-emerald-50 text-emerald-700 border-emerald-200',
    },
    {
      title: 'Global State Root',
      value: formatHash(stateRoot || '0x0000000000000000', 6),
      sub: '256-bit Sparse Merkle Tree',
      icon: Layers,
      iconBg: 'bg-indigo-50 text-indigo-600 border border-indigo-100',
      badge: 'Deterministic',
      badgeColor: 'bg-indigo-50 text-indigo-700 border-indigo-200',
    },
    {
      title: 'Active Validators',
      value: validatorsCount.toString(),
      sub: 'Consensus Quorum: >2/3',
      icon: Shield,
      iconBg: 'bg-purple-50 text-purple-600 border border-purple-100',
      badge: '100% Honest',
      badgeColor: 'bg-purple-50 text-purple-700 border-purple-200',
    },
    {
      title: 'Pending Mempool',
      value: mempoolSize.toString(),
      sub: 'Fee-per-Byte Ranked',
      icon: Zap,
      iconBg: 'bg-amber-50 text-amber-600 border border-amber-100',
      badge: 'Zero Spammed',
      badgeColor: 'bg-amber-50 text-amber-700 border-amber-200',
    },
  ]

  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
      {stats.map((stat, i) => {
        const Icon = stat.icon
        return (
          <div
            key={i}
            className="group relative overflow-hidden rounded-2xl border border-slate-200/80 bg-white p-5 shadow-card hover:shadow-card-hover hover:border-slate-300 transition-all duration-200"
          >
            <div className="flex items-start justify-between">
              <div className={`flex h-11 w-11 items-center justify-center rounded-xl ${stat.iconBg} transition-transform group-hover:scale-105`}>
                <Icon className="h-5 w-5" />
              </div>
              <span className={`rounded-full px-2 py-0.5 text-[10px] font-bold border ${stat.badgeColor}`}>
                {stat.badge}
              </span>
            </div>

            <div className="mt-4">
              <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">{stat.title}</span>
              <div className="text-xl font-bold font-mono text-slate-900 mt-0.5 tracking-tight">{stat.value}</div>
              <span className="text-xs text-slate-400 mt-1 block font-sans">{stat.sub}</span>
            </div>
          </div>
        )
      })}
    </div>
  )
}
