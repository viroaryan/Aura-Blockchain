'use client'

import React, { useEffect, useState } from 'react'
import { OmniSearch } from '@/components/OmniSearch'
import { StatsOverview } from '@/components/StatsOverview'
import { LatestBlocksTable } from '@/components/LatestBlocksTable'
import { LatestTxTable } from '@/components/LatestTxTable'
import { Block, getNodeInfo, getBlockByHeight, Transaction } from '@/lib/rpc'
import { Sparkles, Shield, Cpu, Network } from 'lucide-react'

export default function Home() {
  const [height, setHeight] = useState<number>(1)
  const [stateRoot, setStateRoot] = useState<string>('0x7a8b9c...d4e5f6')
  const [validatorsCount, setValidatorsCount] = useState<number>(1)
  const [mempoolSize, setMempoolSize] = useState<number>(0)
  const [peerCount, setPeerCount] = useState<number>(1)
  const [blocks, setBlocks] = useState<Block[]>([])
  const [transactions, setTransactions] = useState<Transaction[]>([])

  useEffect(() => {
    async function fetchData() {
      try {
        const info = await getNodeInfo()
        setHeight(info.latest_height)
        setStateRoot(info.state_root)
        setMempoolSize(info.mempool_size)
        setPeerCount(info.peer_count)

        const fetchedBlocks: Block[] = []
        const fetchedTxs: Transaction[] = []

        const start = Math.max(0, info.latest_height - 5)
        for (let h = info.latest_height; h >= start && h >= 0; h--) {
          try {
            const b = await getBlockByHeight(h)
            fetchedBlocks.push(b)
            if (b.transactions) {
              fetchedTxs.push(...b.transactions)
            }
          } catch (e) {}
        }
        setBlocks(fetchedBlocks)
        setTransactions(fetchedTxs.slice(0, 10))
      } catch (err) {
        const sampleBlock: Block = {
          header: {
            version: 1,
            chain_id: 'aura-mainnet-1',
            height: 1,
            round: 0,
            prev_hash: '0000000000000000000000000000000000000000000000000000000000000000',
            merkle_root: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
            state_root: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
            validator_set_hash: 'c89329e46a7e029c4176cf3607066ba6f98ecbe1e7f3c1356f91f391ad5b565a',
            timestamp: Math.floor(Date.now() / 1000),
            proposer: 'aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m',
            signature: '0x3a4b...5c6d',
          },
          transactions: [
            {
              sender: 'aura1qyqszqgpqyqszqgpqyqszqgpqyqszqgppwh42m',
              recipient: 'aura1f4g7j2k9l0m3n5p7r9t1v3x5z7b9d1f3h5j7k',
              amount: 50_000_000,
              fee: 1_000,
              nonce: 1,
              tx_type: 'Transfer',
              payload: [],
              pubkey: '0x1111...2222',
              signature: '0x3333...4444',
            },
          ],
        }
        setBlocks([sampleBlock])
        setTransactions(sampleBlock.transactions)
      }
    }

    fetchData()
    const interval = setInterval(fetchData, 3000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div className="space-y-10">
      {/* Hero & Search */}
      <div className="flex flex-col items-center justify-center text-center space-y-4 pt-6 pb-2">
        <div className="inline-flex items-center gap-2 rounded-full border border-emerald-200 bg-emerald-50/80 px-3 py-1 text-xs font-bold text-emerald-800 shadow-2xs">
          <Sparkles className="h-3.5 w-3.5 text-emerald-600" />
          <span>Proof-of-Stake BFT & Decentralized VPN Telemetry</span>
        </div>

        <h1 className="text-3xl sm:text-5xl font-extrabold tracking-tight text-slate-900 max-w-3xl">
          The Next-Gen <span className="bg-gradient-to-r from-emerald-600 via-teal-600 to-indigo-600 bg-clip-text text-transparent">Aura Blockchain</span> Explorer
        </h1>

        <p className="max-w-2xl text-sm sm:text-base text-slate-600 font-medium">
          Deterministic 2-Phase BFT finality, 256-bit Sparse Merkle Tree state verification, and encrypted peer-to-peer mesh networking.
        </p>

        <div className="w-full pt-4">
          <OmniSearch />
        </div>
      </div>

      {/* Network Stats */}
      <StatsOverview
        height={height}
        stateRoot={stateRoot}
        validatorsCount={validatorsCount}
        mempoolSize={mempoolSize}
        peerCount={peerCount}
      />

      {/* Tables Grid */}
      <div className="grid grid-cols-1 gap-8 lg:grid-cols-2">
        <LatestBlocksTable blocks={blocks} />
        <LatestTxTable transactions={transactions} />
      </div>
    </div>
  )
}
