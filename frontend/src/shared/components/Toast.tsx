import { Toaster } from "sonner";
import { useTheme } from "../contexts/ThemeContext";

const Toast = () => {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  return (
    <Toaster
      richColors={false}
      position="top-right"
      closeButton={true}
      duration={3000}
      visibleToasts={1}
      expand={false}
      toastOptions={{
        unstyled: true,
        className: `${theme === "dark" ? "bg-[#2d2820]/90 text-[#f5efe5]" : "bg-white/95 text-[#2d2820]"} backdrop-blur-[40px] w-[340px] flex flex-row text-md py-3 px-4 rounded-md border ${theme === "dark" ? "border-white/20" : "border-white/30"} shadow-[0_8px_32px_rgba(0,0,0,0.15)]`,
        classNames: {
          closeButton:
            "order-last ml-auto cursor-pointer hover:opacity-70 transition-opacity",
          icon: "mr-1 mt-0.5 flex-shrink-0",
          description: "mt-0.5 text-sm",
          success: `!bg-transparent !border-none !shadow-none !text-inherit`,
          error: `!bg-transparent !border-none !shadow-none !text-inherit`,
        },
        className: `${isDark ? 'bg-[#2d2820] text-[#e8dfd0] border-white/15' : 'bg-[#ede3d0] text-[#2d2820] border-[#c9983a]/30'} backdrop-blur-[40px] w-[340px] flex flex-row text-md py-3 px-4 rounded-[14px] border-2 shadow-lg`,
        classNames: {
          closeButton: "order-last ml-auto cursor-pointer rounded-[10px] p-1 hover:opacity-80 transition-opacity",
          icon: "mr-2 mt-0.5 flex-shrink-0",
          description: "mt-0.5 text-sm",
          success: isDark
            ? '!border-[#c9983a]/60 !bg-[#3a3228] !text-[#e8dfd0] [&_svg]:text-[#c9983a] shadow-[0_4px_20px_rgba(201,152,58,0.25)]'
            : '!border-[#c9983a]/70 !bg-[#f5eed8] !text-[#2d2820] [&_svg]:text-[#a67c2e] shadow-[0_4px_20px_rgba(201,152,58,0.2)]',
          error: isDark
            ? '!border-red-500/50 !bg-[#3a3228] [&_[data-icon]]:text-red-400'
            : '!border-red-500/50 !bg-[#f5eed8] [&_[data-icon]]:text-red-600',
        }
      }}
    />
  );
};

export default Toast;
