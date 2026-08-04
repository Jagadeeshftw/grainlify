import { useMediaQuery } from "../../../shared/hooks/useMediaQuery";

export function useIsMobile(): boolean {
  return useMediaQuery("(max-width: 767px)");
}
