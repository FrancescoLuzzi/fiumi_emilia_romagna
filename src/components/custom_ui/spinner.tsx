import * as react from "react";
import { Loader, LucideProps } from "lucide-react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const spinnerVariants = cva("animate-spin-slow", {
  variants: {
    variant: {
      default: "",
      destructive: "text-destructive",
      primary: "text-primary",
      secondary: "text-secondary",
    },
  },
  defaultVariants: {
    variant: "default",
  },
});

export interface SpinnerProps
  extends LucideProps,
    VariantProps<typeof spinnerVariants> {}

export const Spinner = react.forwardRef<SVGElement, SpinnerProps>(
  ({ className, variant, ...props }) => {
    return (
      <Loader
        className={cn(spinnerVariants({ variant }), className)}
        {...props}
      />
    );
  },
);
Spinner.displayName = "Spinner";
