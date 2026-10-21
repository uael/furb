import * as Tooltip from "@radix-ui/react-tooltip";
import type { ReactNode } from "react";

/** A tip that shows over its child after a short wait: a few words that say what the child is or does. */
export function Tip({
  text,
  children,
  side = "top",
}: {
  text: ReactNode;
  children: ReactNode;
  side?: "top" | "right" | "bottom" | "left";
}) {
  if (!text) return <>{children}</>;
  return (
    <Tooltip.Root delayDuration={350}>
      <Tooltip.Trigger asChild>{children}</Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          side={side}
          sideOffset={6}
          collisionPadding={12}
          className="z-50 max-w-[360px] rounded-lg border border-hair bg-raised px-2.5 py-1.5 text-[12px] leading-snug text-text shadow-[var(--shadow)]"
        >
          {text}
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}
