import { Toaster } from "sonner";
import { useTheme } from "../contexts/ThemeContext";

const Toast = () => {
  const { theme } = useTheme();
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
      }}
    />
  );
};

export default Toast;
