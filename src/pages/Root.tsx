import { Outlet } from "react-router-dom";
import { Toaster } from "@/components/ui/sonner";
import { TOAST_DURATION } from "@/constants";
import { ThemeSwitch } from "@/components/theme-switch";

export function Root() {
  return (
    <div className="flex size-full flex-col">
      <nav className="flex max-w-full flex-row">
        <ThemeSwitch />
      </nav>
      <Outlet />
      <Toaster
        className="z-0"
        position="bottom-right"
        duration={TOAST_DURATION}
        richColors
        closeButton
      />
    </div>
  );
}
