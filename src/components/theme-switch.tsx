import { Toggle } from "@/components/ui/toggle";
import { Theme, useTheme } from "@/contexts/theme";
import { cn } from "@/lib/utils";
import { SunIcon, MoonIcon } from "lucide-react";

export const ThemeSwitch = () => {
  const { theme, toggleTheme } = useTheme();
  const enabledTheme = (wantedTheme: Theme) => {
    return () => (wantedTheme === theme ? "opacity-100" : "opacity-20");
  };
  return (
    <Toggle
      variant={"outline"}
      className="space-x-2"
      onPressedChange={() => toggleTheme()}
    >
      <MoonIcon className={cn("size-6", enabledTheme("dark")())} />
      <SunIcon className={cn("size-6", enabledTheme("light")())} />
    </Toggle>
  );
};
