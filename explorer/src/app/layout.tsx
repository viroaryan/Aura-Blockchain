import './globals.css'
import { Plus_Jakarta_Sans, JetBrains_Mono } from 'next/font/google'
import { Navbar } from '@/components/Navbar'

const sansFont = Plus_Jakarta_Sans({
  subsets: ['latin'],
  variable: '--font-sans',
  weight: ['400', '500', '600', '700', '800'],
})

const monoFont = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
  weight: ['400', '500', '600', '700'],
})

export const metadata = {
  title: 'Aura Network | Proof-of-Stake Blockchain & Mesh dVPN',
  description: 'Enterprise-grade blockchain explorer, decentralized web wallet, and encrypted remote hotspot sharing protocol.',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" className={`${sansFont.variable} ${monoFont.variable}`}>
      <body className="min-h-screen font-sans bg-[#f8fafc] text-slate-900 antialiased selection:bg-emerald-500/20 selection:text-emerald-900">
        <Navbar />
        <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6">
          {children}
        </main>
      </body>
    </html>
  )
}
