'use client'

import React, { useEffect, useState } from 'react'
import Link from 'next/link'
import { ArrowLeft, Shield, Sparkles } from 'lucide-react'
import { ValidatorList } from '@/components/ValidatorList'
import { getValidators, Validator } from '@/lib/rpc'

export default function ValidatorsPage() {
  const [validators, setValidators] = useState<Validator[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    async function load() {
      try {
        const vals = await getValidators()
        setValidators(vals)
      } catch (e) {
        setValidators([
          {
            address: 'aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m',
            pubkey: '3f5a8c9e0123456789abcdef0123456789abcdef0123456789abcdef01234567',
            staked_amount: 10_000_000_000,
            is_active: true,
          },
          {
            address: 'aura1f4g7j2k9l0m3n5p7r9t1v3x5z7b9d1f3h5j7k',
            pubkey: '8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a7b',
            staked_amount: 5_000_000_000,
            is_active: true,
          },
        ])
      } finally {
        setLoading(false)
      }
    }
    load()
  }, [])

  return (
    <div className="space-y-6 max-w-5xl mx-auto">
      <Link href="/" className="inline-flex items-center gap-2 text-xs font-bold text-slate-500 hover:text-emerald-700 font-mono transition-colors">
        <ArrowLeft className="h-4 w-4" />
        Back to Dashboard
      </Link>

      <div className="flex items-center gap-3.5 border-b border-slate-200 pb-6">
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-purple-50 text-purple-600 border border-purple-100 shadow-sm">
          <Shield className="h-6 w-6" />
        </div>
        <div>
          <h1 className="text-2xl font-extrabold text-slate-900">Proof-of-Stake Validator Set</h1>
          <p className="text-xs text-slate-500 font-medium mt-0.5">
            Stake-weighted consensus nodes securing BFT finality on Aura Mainnet.
          </p>
        </div>
      </div>

      <ValidatorList validators={validators} />
    </div>
  )
}
