import type { Metadata } from 'next'
import { Geist, Geist_Mono } from 'next/font/google'
import './globals.css'

const geistSans = Geist({
  variable: '--font-geist-sans',
  subsets: ['latin'],
})

const geistMono = Geist_Mono({
  variable: '--font-geist-mono',
  subsets: ['latin'],
})

export const metadata: Metadata = {
  title: 'Claude Tabs - Never Miss Your Turn',
  description: 'A tab-based terminal manager for Claude Code. Run parallel sessions, get notified when Claude needs you, and never lose context again.',
  icons: {
    icon: '/favicon.svg',
  },
  openGraph: {
    title: 'Claude Tabs - Never Miss Your Turn',
    description: 'A tab-based terminal manager for Claude Code. Run parallel sessions, get notified when Claude needs you.',
    url: 'https://claudetabs.app',
    siteName: 'Claude Tabs',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Claude Tabs - Never Miss Your Turn',
    description: 'A tab-based terminal manager for Claude Code. Run parallel sessions, get notified when Claude needs you.',
  },
}

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className={`${geistSans.variable} ${geistMono.variable} font-sans antialiased`}>
        {children}
      </body>
    </html>
  )
}
