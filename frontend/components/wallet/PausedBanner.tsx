"use client";

import { ShieldAlert } from "lucide-react";
import { useStore } from "@/lib/state/store";

/**
 * Full-width banner shown when the contract is in emergency-stop state.
 * Displayed above all app content so it is impossible to miss.
 * Read operations still work; all write actions are blocked on-chain.
 */
export function PausedBanner() {
  const { contractPaused } = useStore();
  if (!contractPaused) return null;

  return (
    <div
      role="alert"
      aria-live="assertive"
      className="w-full bg-red-600 text-white px-4 py-3 flex items-center gap-3 text-sm font-medium"
    >
      <ShieldAlert size={18} className="shrink-0" />
      <span>
        Emergency stop active — all write operations are currently disabled.
        The contract is in read-only mode. Contact a guardian to resume normal operation.
      </span>
    </div>
  );
}
