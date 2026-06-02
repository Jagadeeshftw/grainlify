import React from 'react'
import { Toaster, toast, useSonner } from 'sonner'
import { useTheme } from '../contexts/ThemeContext'

const MAX_VISIBLE_TOASTS = 3
const DEFAULT_TOAST_DURATION = 5000

const Toast = () => {
  const { theme } = useTheme()
  const isDark = theme === 'dark'
  const { toasts } = useSonner()
  const queuedCount = Math.max(0, toast.getToasts().length - MAX_VISIBLE_TOASTS)

  return (
    <div className="relative">
      <Toaster
        theme={theme as 'light' | 'dark' | 'system'}
        position="bottom-right"
        visibleToasts={MAX_VISIBLE_TOASTS}
        gap={12}
        duration={DEFAULT_TOAST_DURATION}
        closeButton
        className="grainlify-toast-viewport"
        containerAriaLabel="Application notifications"
        toastOptions={{
          unstyled: true,
          closeButtonAriaLabel: 'Dismiss notification',
          className: `backdrop-blur-[40px] w-[340px] flex flex-row text-md py-3 px-4 rounded-[18px] border-2 shadow-[0_25px_50px_rgba(0,0,0,0.18)] transition-all motion-safe:duration-300 motion-safe:ease-out motion-reduce:transition-none ${isDark ? 'bg-[#2d2820] text-[#e8dfd0] border-white/15' : 'bg-[#ede3d0] text-[#2d2820] border-[#c9983a]/30'}`,
          classNames: {
            toast: 'grainlify-toast motion-safe:transform-none motion-reduce:transition-none',
            title: 'font-semibold text-sm leading-5',
            description: 'mt-0.5 text-sm leading-5',
            icon: 'mr-3 mt-0.5 flex-shrink-0',
            closeButton: 'order-last ml-auto rounded-xl p-1 hover:opacity-90 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-600',
            loader: 'h-1 mt-3 overflow-hidden rounded-full bg-current/20',
            actionButton: 'ml-3 rounded-full border border-current bg-transparent px-3 py-1 text-sm font-semibold transition hover:bg-current/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-600',
            success: isDark
              ? '!border-[#c9983a]/60 !bg-[#3a3228] !text-[#e8dfd0] [&_svg]:text-[#c9983a]'
              : '!border-[#c9983a]/70 !bg-[#f5eed8] !text-[#2d2820] [&_svg]:text-[#a67c2e]',
            error: isDark
              ? '!border-red-500/50 !bg-[#3a3228] [&_[data-icon]]:text-red-400'
              : '!border-red-500/50 !bg-[#fef2f2] [&_[data-icon]]:text-red-600',
            warning: isDark
              ? '!border-[#f59e0b]/50 !bg-[#3a2b0d] [&_[data-icon]]:text-[#f59e0b]'
              : '!border-[#f59e0b]/30 !bg-[#fffaeb] [&_[data-icon]]:text-[#b45309]',
            info: isDark
              ? '!border-[#2563eb]/50 !bg-[#1d2e4f] [&_[data-icon]]:text-[#93c5fd]'
              : '!border-[#3b82f6]/60 !bg-[#eff6ff] [&_[data-icon]]:text-[#2563eb]',
            loading: isDark
              ? '!border-[#6366f1]/50 !bg-[#1f1f35] [&_[data-icon]]:text-[#a5b4fc]'
              : '!border-[#6366f1]/60 !bg-[#eef2ff] [&_[data-icon]]:text-[#4338ca]',
            action: isDark
              ? '!border-[#f1b400]/60 !bg-[#3a321e] [&_[data-icon]]:text-[#f1b400]'
              : '!border-[#f1b400]/60 !bg-[#fff7e0] [&_[data-icon]]:text-[#a67c2e]',
          }
        }}
      />
      {queuedCount > 0 && (
        <div className="grainlify-toast-queue-badge" aria-hidden="true">
          +{queuedCount}
        </div>
      )}
    </div>
  )
}

export default Toast
