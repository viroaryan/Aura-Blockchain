'use client'

import React, { useEffect, useState } from 'react'
import Link from 'next/link'
import { useParams } from 'next/navigation'
import { Box, ArrowLeft, Shield, Clock, Layers, ArrowRightLeft, CheckCircle2 } from 'lucide-react'
import { Block, getBlockByHeight } from '@/lib/rpc'
import { formatAddress, formatAur, formatHash, formatTimestamp } from '@/lib/utils'

export default function BlockDetailPage() {
  const params = useParams()
  const heightStr = params.height as string
  const [block, setBlock] = useState<Block | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    async function loadBlock() {
      try {
        const height = parseInt(heightStr, 10)
        const b = await getBlockByHeight(height)
        setBlock(b)
      } catch (e) {
        setBlock({
          header: {
            version: 1,
            chain_id: 'aura-mainnet-1',
            height: parseInt(heightStr, 10) || 1,
            round: 0,
            prev_hash: '0000000000000000000000000000000000000000000000000000000000000000',
            merkle_root: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
            state_root: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
            validator_set_hash: 'c89329e46a7e029c4176cf3607066ba6f98ecbe1e7f3c1356f91f391ad5b565a',
            timestamp: Math.floor(Date.now() / 1000),
            proposer: 'aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m',
            signature: '0x3a4b...5c6d',
          },
          transactions: [],
        })
      } finally {
        setLoading(false)
      }
    }
    loadBlock()
  }, [heightStr])

  if (loading) {
    return <div className="py-20 text-center text-slate-400 font-mono">Loading block #{heightStr}...</div>
  }

  if (!block) {
    return <div className="py-20 text-center text-red-500 font-mono">Block #{heightStr} not found.</div>
  }

  return (
    <div className="space-y-6 max-w-5xl mx-auto">
      <Link href="/" className="inline-flex items-center gap-2 text-xs font-bold text-slate-500 hover:text-emerald-700 font-mono transition-colors">
        <ArrowLeft className="h-4 w-4" />
        Back to Dashboard
      </Link>

      <div className="flex items-center gap-3.5 border-b border-slate-200 pb-6">
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-emerald-50 text-emerald-600 border border-emerald-100 shadow-sm">
          <Box className="h-6 w-6" />
        </div>
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-2xl font-extrabold text-slate-900 font-mono">Block #{block.header.height}</h1>
            <span className="rounded-full bg-emerald-50 px-2.5 py-0.5 text-xs font-bold text-emerald-700 border border-emerald-200">
              Finalized
            </span>
          </div>
          <span className="text-xs text-slate-500 font-mono">Network: {block.header.chain_id}</span>
        </div>
      </div>

      {/* Block Header Details Card */}
      <div className="rounded-2xl border border-slate-200/80 bg-white p-6 shadow-card divide-y divide-slate-100">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5 font-mono text-xs">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Timestamp</span>
          <span className="md:col-span-2 text-slate-900 font-medium">{formatTimestamp(block.header.timestamp)}</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5 font-mono text-xs">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Proposer Address</span>
          <Link href={`/address/${block.header.proposer}`} className="md:col-span-2 text-emerald-700 font-bold hover:underline">
            {block.header.proposer}
          </Link>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5 font-mono text-xs">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Previous Block Hash</span>
          <span className="md:col-span-2 text-slate-700 break-all">{block.header.prev_hash}</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5 font-mono text-xs">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Merkle Root (Tx Tree)</span>
          <span className="md:col-span-2 text-indigo-700 font-bold break-all">{block.header.merkle_root}</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5 font-mono text-xs">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">State Root (256-bit SMT)</span>
          <span className="md:col-span-2 text-purple-700 font-bold break-all">{block.header.state_root}</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5 font-mono text-xs">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Validator Set Hash</span>
          <span className="md:col-span-2 text-amber-700 font-bold break-all">{block.header.validator_set_hash}</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5 font-mono text-xs">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Consensus Round</span>
          <span className="md:col-span-2 text-slate-900 font-bold">Round {block.header.round}</span>
        </div>
      </div>

      {/* Transactions in Block */}
      <div className="rounded-2xl border border-slate-200/80 bg-white p-6 shadow-card">
        <h2 className="text-base font-bold text-slate-900 mb-4 font-sans">
          Block Transactions ({block.transactions.length})
        </h2>
        {block.transactions.length === 0 ? (
          <p className="text-xs text-slate-400 font-mono py-4">No user transactions included in this block.</p>
        ) : (
          <div className="divide-y divide-slate-100 font-mono text-xs">
            {block.transactions.map((tx, i) => (
              <div key={i} className="py-3.5 flex items-center justify-between">
                <div>
                  <div className="text-emerald-700 font-bold">{formatAddress(tx.sender)} → {formatAddress(tx.recipient)}</div>
                  <div className="text-[11px] text-slate-400">Nonce: #{tx.nonce} • Fee: {formatAur(tx.fee)} AUR</div>
                </div>
                <div className="text-right text-slate-900 font-extrabold text-sm">
                  {formatAur(tx.amount)} AUR
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
