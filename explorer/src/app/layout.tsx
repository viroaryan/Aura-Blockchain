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
  title: 'Aura Mesh | Free Global Communication, 4K Media Drop & Internet Relay',
  description: '100% Free peer-to-peer messaging, direct binary 4K video & media sharing, and global encrypted hotspot internet relay with zero server storage.',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" className={`${sansFont.variable} ${monoFont.variable}`}>
      <body className="min-h-screen font-sans bg-[#f8fafd] text-slate-900 antialiased selection:bg-emerald-500/20 selection:text-emerald-900 pb-20 md:pb-8">
        <Navbar />
        <main className="mx-auto max-w-7xl px-4 py-6 sm:px-6">
          {children}
        </main>
      </body>
    </html>
  )
}
