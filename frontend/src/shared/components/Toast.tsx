import React from 'react'
import { Toaster } from 'sonner'
import { useTheme } from '../contexts/ThemeContext'

const Toast = () => {
  const { theme } = useTheme();
  return (
    <Toaster
      richColors={false}
      position="top-right"
      closeButton={true}
      duration={3000}
      toastOptions={{
        unstyled: true,
        className: `${theme === 'dark' ? 'bg-[#2d2820]/10 text-[#e8dfd0]' : 'bg-[#e8dfd0]/10 text-[#2d2820]'}  backdrop-blur-[40px] w-[340px] flex flex-row text-md py-3 px-4 rounded-md border border-white/15`,
        classNames: {
          closeButton: "order-last ml-auto cursor-pointer",
          icon: "mr-1 mt-0.5",
          description: "mt-0.5 text-sm",
          success: 'border border-[#c9983a]/50 shadow-[0_2px_20px_#c9983a4D]'
        }
      }}
    />
  )
}

export default Toast
