'use client'

import { useState } from 'react'

interface CopyButtonProps {
  text: string
  variant?: 'primary' | 'cta'
}

export function CopyButton({ text, variant = 'primary' }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  if (variant === 'cta') {
    return (
      <button
        onClick={handleCopy}
        className="px-10 py-5 bg-gradient-to-b from-accent to-accent-dim border-none rounded-xl cursor-pointer transition-all hover:scale-105 hover:shadow-lg hover:shadow-accent/20"
        aria-label="Copy install command"
      >
        <code className="text-lg text-text-primary font-mono font-medium">
          {text}
        </code>
      </button>
    )
  }

  return (
    <button
      onClick={handleCopy}
      className="flex flex-col items-center gap-1.5 px-8 py-4 bg-gradient-to-b from-zinc-800 to-bg-secondary border border-zinc-700 rounded-xl cursor-pointer transition-all hover:border-zinc-600"
      aria-label="Copy install command"
    >
      <code className="text-base text-text-primary font-mono">
        {text}
      </code>
      <span className="text-xs text-text-muted">
        {copied ? 'Copied!' : 'Click to copy'}
      </span>
    </button>
  )
}
