'use client'

import React from 'react'
import Link from 'next/link'
import { Box, Clock, ChevronRight, CheckCircle2 } from 'lucide-react'
import { Block } from '@/lib/rpc'
import { formatAddress, formatTimestamp } from '@/lib/utils'

export function LatestBlocksTable({ blocks }: { blocks: Block[] }) {
  return (
    <div className="rounded-2xl border border-slate-200/80 bg-white p-6 shadow-card">
      <div className="flex items-center justify-between border-b border-slate-100 pb-4 mb-2">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-50 text-emerald-600 border border-emerald-100">
            <Box className="h-4 w-4" />
          </div>
          <div>
            <h2 className="text-sm font-bold text-slate-900">Latest Blocks</h2>
            <span className="text-[11px] text-slate-400">Real-time PoS-BFT finality</span>
          </div>
        </div>
        <span className="rounded-full bg-slate-50 border border-slate-200 px-2.5 py-0.5 text-[11px] font-mono text-slate-600">
          Live Sync
        </span>
      </div>

      <div className="divide-y divide-slate-100">
        {blocks.length === 0 ? (
          <div className="py-12 text-center text-sm text-slate-400 font-mono">
            Waiting for next block proposal...
          </div>
        ) : (
          blocks.map((b, i) => (
            <div
              key={i}
              className="flex items-center justify-between py-3.5 px-2 hover:bg-slate-50/80 rounded-xl transition-all group"
            >
              <div className="flex items-center gap-3.5">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-slate-100 text-slate-800 font-mono text-xs font-bold border border-slate-200/60 group-hover:bg-emerald-50 group-hover:text-emerald-700 group-hover:border-emerald-200 transition-colors">
                  #{b.header.height}
                </div>
                <div>
                  <Link
                    href={`/blocks/${b.header.height}`}
                    className="font-mono text-sm font-bold text-slate-900 hover:text-emerald-600 transition-colors flex items-center gap-1"
                  >
                    <span>Block #{b.header.height}</span>
                    <ChevronRight className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                  </Link>
                  <div className="flex items-center gap-2 text-xs text-slate-500 mt-0.5">
                    <span>Proposer:</span>
                    <Link
                      href={`/address/${b.header.proposer}`}
                      className="font-mono text-emerald-600 hover:underline"
                    >
                      {formatAddress(b.header.proposer, 5)}
                    </Link>
                  </div>
                </div>
              </div>

              <div className="text-right">
                <span className="rounded-md bg-slate-100 px-2 py-0.5 text-[11px] font-mono font-medium text-slate-700 border border-slate-200/50">
                  {b.transactions.length} txs
                </span>
                <div className="mt-1 flex items-center justify-end gap-1 text-[11px] text-slate-400 font-mono">
                  <Clock className="h-3 w-3" />
                  <span>{formatTimestamp(b.header.timestamp)}</span>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
