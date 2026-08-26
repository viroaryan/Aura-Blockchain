'use client'

import React from 'react'
import Link from 'next/link'
import { Shield, CheckCircle2 } from 'lucide-react'
import { Validator } from '@/lib/rpc'
import { formatAddress, formatAur, formatHash } from '@/lib/utils'

export function ValidatorList({ validators }: { validators: Validator[] }) {
  const totalStake = validators.reduce((acc, v) => acc + v.staked_amount, 0)

  return (
    <div className="rounded-2xl border border-slate-200/80 bg-white p-6 shadow-card">
      <div className="flex items-center justify-between border-b border-slate-100 pb-4 mb-4">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-purple-50 text-purple-600 border border-purple-100">
            <Shield className="h-4 w-4" />
          </div>
          <div>
            <h2 className="text-sm font-bold text-slate-900">Active Validator Set</h2>
            <span className="text-[11px] text-slate-400">PoS-BFT consensus signers</span>
          </div>
        </div>
        <span className="rounded-full bg-slate-50 border border-slate-200 px-3 py-0.5 text-xs font-mono font-bold text-slate-700">
          Total Staked: {formatAur(totalStake)} AUR
        </span>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-left text-sm font-mono">
          <thead className="text-[11px] uppercase tracking-wider text-slate-400 border-b border-slate-100 bg-slate-50/50">
            <tr>
              <th className="py-3 px-4 font-bold">Validator Address</th>
              <th className="py-3 px-4 font-bold">Public Key</th>
              <th className="py-3 px-4 text-right font-bold">Staked Balance</th>
              <th className="py-3 px-4 text-right font-bold">Voting Share</th>
              <th className="py-3 px-4 text-center font-bold">Status</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {validators.length === 0 ? (
              <tr>
                <td colSpan={5} className="py-12 text-center text-slate-400 font-mono">
                  No active validators found in state.
                </td>
              </tr>
            ) : (
              validators.map((v, i) => {
                const share = totalStake > 0 ? ((v.staked_amount / totalStake) * 100).toFixed(2) : '100.00'
                return (
                  <tr key={i} className="hover:bg-slate-50/80 transition-all">
                    <td className="py-3.5 px-4 font-bold text-emerald-700">
                      <Link href={`/address/${v.address}`} className="hover:underline">
                        {v.address}
                      </Link>
                    </td>
                    <td className="py-3.5 px-4 text-slate-500">
                      {v.pubkey ? formatHash(v.pubkey, 8) : 'N/A'}
                    </td>
                    <td className="py-3.5 px-4 text-right text-slate-900 font-extrabold">
                      {formatAur(v.staked_amount)} AUR
                    </td>
                    <td className="py-3.5 px-4 text-right text-purple-700 font-extrabold">
                      {share}%
                    </td>
                    <td className="py-3.5 px-4 text-center">
                      <span className="inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2.5 py-0.5 text-xs font-bold text-emerald-700 border border-emerald-200">
                        <CheckCircle2 className="h-3 w-3" />
                        Active
                      </span>
                    </td>
                  </tr>
                )
              })
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
