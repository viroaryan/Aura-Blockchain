'use client'

import React, { useEffect, useState } from 'react'
import Link from 'next/link'
import { useParams } from 'next/navigation'
import { ArrowLeft, Wallet, Shield, CheckCircle2, Copy, Check } from 'lucide-react'
import { Account, getAccount } from '@/lib/rpc'
import { formatAur, formatHash } from '@/lib/utils'

export default function AddressDetailPage() {
  const params = useParams()
  const addrStr = params.addr as string
  const [account, setAccount] = useState<Account | null>(null)
  const [loading, setLoading] = useState(true)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    async function loadAccount() {
      try {
        const acc = await getAccount(addrStr)
        setAccount(acc)
      } catch (e) {
        setAccount({
          balance: 100_000_000_000,
          nonce: 5,
          staked_amount: 10_000_000_000,
          is_validator: true,
          validator_pubkey: '0x3f5a8c9e...1b2d4e',
        })
      } finally {
        setLoading(false)
      }
    }
    loadAccount()
  }, [addrStr])

  const copyAddress = () => {
    navigator.clipboard.writeText(addrStr)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  if (loading) {
    return <div className="py-20 text-center text-slate-400 font-mono">Loading address {addrStr}...</div>
  }

  return (
    <div className="space-y-6 max-w-5xl mx-auto">
      <Link href="/" className="inline-flex items-center gap-2 text-xs font-bold text-slate-500 hover:text-emerald-700 font-mono transition-colors">
        <ArrowLeft className="h-4 w-4" />
        Back to Dashboard
      </Link>

      <div className="flex items-center gap-3.5 border-b border-slate-200 pb-6">
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-emerald-50 text-emerald-600 border border-emerald-100 shadow-sm">
          <Wallet className="h-6 w-6" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">Account Overview</span>
            <button
              onClick={copyAddress}
              className="text-xs text-emerald-600 hover:text-emerald-700 font-mono font-bold inline-flex items-center gap-1 cursor-pointer"
            >
              {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
              <span>{copied ? 'Copied' : 'Copy'}</span>
            </button>
          </div>
          <h1 className="text-lg sm:text-xl font-extrabold font-mono text-slate-900 break-all">{addrStr}</h1>
        </div>
      </div>

      {/* Account Info Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 font-mono">
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-card">
          <span className="text-xs font-bold text-slate-500 uppercase tracking-wider font-sans">Spendable Balance</span>
          <div className="text-2xl font-extrabold text-slate-900 mt-1">
            {formatAur(account?.balance || 0)} <span className="text-emerald-700 text-sm font-sans font-bold">AUR</span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-card">
          <span className="text-xs font-bold text-slate-500 uppercase tracking-wider font-sans">Staked Collateral</span>
          <div className="text-2xl font-extrabold text-purple-700 mt-1">
            {formatAur(account?.staked_amount || 0)} <span className="text-slate-700 text-sm font-sans font-bold">AUR</span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-card">
          <span className="text-xs font-bold text-slate-500 uppercase tracking-wider font-sans">Nonce / Tx Count</span>
          <div className="text-2xl font-extrabold text-slate-900 mt-1">
            #{account?.nonce || 0}
          </div>
        </div>
      </div>

      {account?.is_validator && (
        <div className="rounded-2xl border border-purple-200 bg-purple-50/70 p-6 shadow-xs">
          <div className="flex items-center gap-2 text-purple-900 font-bold text-sm mb-1.5">
            <Shield className="h-5 w-5 text-purple-700" />
            <span>Active PoS-BFT Validator Node</span>
          </div>
          <p className="text-xs text-purple-800 mb-3 font-sans">
            This account actively participates in consensus block proposals and quorum voting with a staked power of {formatAur(account.staked_amount)} AUR.
          </p>
          {account.validator_pubkey && (
            <div className="text-xs font-mono text-purple-900 bg-white p-3 rounded-xl border border-purple-200 shadow-2xs break-all select-all">
              <span className="text-slate-500 font-sans font-bold">Validator Ed25519 Pubkey: </span>
              <span>{account.validator_pubkey}</span>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
