'use client'

import React, { useState } from 'react'
import { useRouter } from 'next/navigation'
import { Search, ArrowRight, CornerDownLeft } from 'lucide-react'

export function OmniSearch() {
  const [query, setQuery] = useState('')
  const router = useRouter()

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    const clean = query.trim()
    if (!clean) return

    if (/^\d+$/.test(clean)) {
      router.push(`/blocks/${clean}`)
    } else if (clean.startsWith('aura1')) {
      router.push(`/address/${clean}`)
    } else if (clean.length === 64 || clean.length === 66) {
      router.push(`/tx/${clean}`)
    } else {
      router.push(`/address/${clean}`)
    }
  }

  return (
    <form onSubmit={handleSearch} className="relative w-full max-w-3xl mx-auto">
      <div className="relative flex items-center shadow-lg shadow-slate-200/50 rounded-2xl transition-all focus-within:shadow-xl focus-within:shadow-emerald-500/10">
        <div className="absolute left-4.5 flex items-center pointer-events-none text-slate-400">
          <Search className="h-5 w-5 text-emerald-600" />
        </div>
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search by Block Height, Tx Hash (0x...), Address (aura1...), or SMT Root..."
          className="w-full rounded-2xl border border-slate-200/90 bg-white py-4 pl-12 pr-28 text-sm text-slate-900 placeholder-slate-400 outline-none transition-all focus:border-emerald-500 focus:ring-4 focus:ring-emerald-500/10 font-mono"
        />
        <div className="absolute right-2.5 flex items-center gap-1.5">
          <button
            type="submit"
            className="flex items-center gap-1.5 rounded-xl bg-emerald-600 px-4 py-2.5 text-xs font-bold text-white shadow-sm hover:bg-emerald-700 transition-all active:scale-95 cursor-pointer"
          >
            <span>Search</span>
            <CornerDownLeft className="h-3 w-3" />
          </button>
        </div>
      </div>

      {/* Quick Search Badges */}
      <div className="flex flex-wrap items-center justify-center gap-2 mt-3 text-[11px] text-slate-500 font-mono">
        <span className="text-slate-400 font-sans">Try searching:</span>
        <button
          type="button"
          onClick={() => { setQuery('1'); router.push('/blocks/1') }}
          className="rounded-md bg-slate-100 px-2 py-0.5 text-slate-600 hover:bg-slate-200 hover:text-slate-900 transition-colors"
        >
          Height: #1
        </button>
        <button
          type="button"
          onClick={() => { setQuery('aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m'); router.push('/address/aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m') }}
          className="rounded-md bg-slate-100 px-2 py-0.5 text-slate-600 hover:bg-slate-200 hover:text-slate-900 transition-colors"
        >
          Genesis Validator
        </button>
      </div>
    </form>
  )
}
