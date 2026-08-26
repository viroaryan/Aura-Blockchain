'use client'

import React, { useEffect, useState } from 'react'
import Link from 'next/link'
import { useParams } from 'next/navigation'
import { ArrowLeft, ArrowRightLeft, CheckCircle2, Copy, Check } from 'lucide-react'
import { formatAddress, formatAur, formatHash } from '@/lib/utils'

export default function TxDetailPage() {
  const params = useParams()
  const txHash = params.hash as string
  const [copied, setCopied] = useState(false)

  const copyHash = () => {
    navigator.clipboard.writeText(txHash)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="space-y-6 max-w-5xl mx-auto">
      <Link href="/" className="inline-flex items-center gap-2 text-xs font-bold text-slate-500 hover:text-emerald-700 font-mono transition-colors">
        <ArrowLeft className="h-4 w-4" />
        Back to Dashboard
      </Link>

      <div className="flex items-center gap-3.5 border-b border-slate-200 pb-6">
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-indigo-50 text-indigo-600 border border-indigo-100 shadow-sm">
          <ArrowRightLeft className="h-6 w-6" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">Transaction Receipt</span>
            <button
              onClick={copyHash}
              className="text-xs text-indigo-600 hover:text-indigo-700 font-mono font-bold inline-flex items-center gap-1 cursor-pointer"
            >
              {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
              <span>{copied ? 'Copied' : 'Copy'}</span>
            </button>
          </div>
          <h1 className="text-lg sm:text-xl font-extrabold font-mono text-slate-900 break-all">{txHash}</h1>
        </div>
      </div>

      <div className="rounded-2xl border border-slate-200/80 bg-white p-6 shadow-card divide-y divide-slate-100 font-mono text-xs">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Status</span>
          <span className="md:col-span-2 inline-flex items-center gap-1.5 font-bold text-emerald-700 bg-emerald-50 px-2.5 py-0.5 rounded-full border border-emerald-200 w-fit">
            <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600" /> Confirmed (PoS-BFT Quorum)
          </span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Transaction Type</span>
          <span className="md:col-span-2 text-slate-900 font-bold">Transfer</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Amount Transferred</span>
          <span className="md:col-span-2 text-slate-900 font-extrabold text-sm">
            50.00 <span className="text-emerald-700 text-xs font-sans font-bold">AUR</span>
          </span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Transaction Fee</span>
          <span className="md:col-span-2 text-slate-600 font-medium">0.001 AUR</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Sender Address</span>
          <Link href="/address/aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m" className="md:col-span-2 text-emerald-700 font-bold break-all hover:underline">
            aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m
          </Link>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-2 py-3.5">
          <span className="font-sans font-bold text-slate-500 uppercase tracking-wider">Recipient Address</span>
          <Link href="/address/aura1f4g7j2k9l0m3n5p7r9t1v3x5z7b9d1f3h5j7k" className="md:col-span-2 text-emerald-700 font-bold break-all hover:underline">
            aura1f4g7j2k9l0m3n5p7r9t1v3x5z7b9d1f3h5j7k
          </Link>
        </div>
      </div>
    </div>
  )
}
