import * as React from "react";

import { Link as DefaultLink } from "react-router-dom";
import { cn } from "@/lib/utils";

export const Link = React.forwardRef<
  React.ElementRef<typeof DefaultLink>,
  React.ComponentPropsWithoutRef<typeof DefaultLink>
>(({ className, children, ...props }, ref) => {
  return (
    <DefaultLink
      ref={ref}
      className={cn("font-medium text-primary hover:underline", className)}
      {...props}
    >
      {children}
    </DefaultLink>
  );
});
Link.displayName = "Link";
