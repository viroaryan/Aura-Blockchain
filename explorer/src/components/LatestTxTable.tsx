'use client'

import React from 'react'
import Link from 'next/link'
import { ArrowRightLeft, ArrowRight, CheckCircle2 } from 'lucide-react'
import { Transaction } from '@/lib/rpc'
import { formatAddress, formatAur } from '@/lib/utils'

export function LatestTxTable({ transactions }: { transactions: Transaction[] }) {
  return (
    <div className="rounded-2xl border border-slate-200/80 bg-white p-6 shadow-card">
      <div className="flex items-center justify-between border-b border-slate-100 pb-4 mb-2">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-50 text-indigo-600 border border-indigo-100">
            <ArrowRightLeft className="h-4 w-4" />
          </div>
          <div>
            <h2 className="text-sm font-bold text-slate-900">Latest Transactions</h2>
            <span className="text-[11px] text-slate-400">Confirmed state transitions</span>
          </div>
        </div>
        <span className="rounded-full bg-slate-50 border border-slate-200 px-2.5 py-0.5 text-[11px] font-mono text-slate-600">
          Settled
        </span>
      </div>

      <div className="divide-y divide-slate-100">
        {transactions.length === 0 ? (
          <div className="py-12 text-center text-sm text-slate-400 font-mono">
            No transactions in recent blocks.
          </div>
        ) : (
          transactions.map((tx, i) => (
            <div
              key={i}
              className="flex items-center justify-between py-3.5 px-2 hover:bg-slate-50/80 rounded-xl transition-all group"
            >
              <div className="flex items-center gap-3.5">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-50 text-emerald-600 font-mono text-xs font-bold border border-emerald-100/80">
                  <ArrowRightLeft className="h-4 w-4" />
                </div>
                <div>
                  <div className="flex items-center gap-1.5 text-xs font-mono">
                    <Link
                      href={`/address/${tx.sender}`}
                      className="font-bold text-slate-900 hover:text-emerald-600 transition-colors"
                    >
                      {formatAddress(tx.sender, 4)}
                    </Link>
                    <ArrowRight className="h-3 w-3 text-slate-400" />
                    <Link
                      href={`/address/${tx.recipient}`}
                      className="font-bold text-slate-900 hover:text-emerald-600 transition-colors"
                    >
                      {formatAddress(tx.recipient, 4)}
                    </Link>
                  </div>
                  <span className="text-[11px] text-slate-400 font-mono block mt-0.5">
                    Nonce #{tx.nonce} • {tx.tx_type}
                  </span>
                </div>
              </div>

              <div className="text-right font-mono">
                <div className="text-sm font-extrabold text-slate-900">
                  {formatAur(tx.amount)} <span className="text-xs font-bold text-emerald-600">AUR</span>
                </div>
                <div className="text-[11px] text-slate-400">
                  Fee: {formatAur(tx.fee)} AUR
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
