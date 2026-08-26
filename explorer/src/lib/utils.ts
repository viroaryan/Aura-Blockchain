import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatAddress(address: string, chars = 6): string {
  if (!address || address.length <= chars * 2) return address
  return `${address.slice(0, chars + 4)}...${address.slice(-chars)}`
}

export function formatHash(hash: string, chars = 8): string {
  if (!hash || hash.length <= chars * 2) return hash
  return `${hash.slice(0, chars)}...${hash.slice(-chars)}`
}

export function formatAur(amount: number | string): string {
  const val = typeof amount === 'string' ? parseFloat(amount) : amount
  return (val / 1_000_000).toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  })
}

export function formatTimestamp(timestamp: number): string {
  const d = new Date(timestamp * 1000)
  return d.toLocaleString()
}
