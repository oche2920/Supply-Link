"use client";

import { useState } from "react";
import type { ElementType } from "react";
import { ShieldAlert, ShieldCheck, ShieldX, Shield, ChevronDown, ChevronUp } from "lucide-react";
import type { RiskScore, RiskLevel } from "@/lib/types";

const LEVEL_CONFIG: Record<
  RiskLevel,
  { label: string; badgeClass: string; iconClass: string; Icon: ElementType }
> = {
  LOW: {
    label: "Low Risk",
    badgeClass: "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-400",
    iconClass: "text-green-500",
    Icon: ShieldCheck,
  },
  MEDIUM: {
    label: "Medium Risk",
    badgeClass: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/40 dark:text-yellow-400",
    iconClass: "text-yellow-500",
    Icon: Shield,
  },
  HIGH: {
    label: "High Risk",
    badgeClass: "bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-400",
    iconClass: "text-orange-500",
    Icon: ShieldAlert,
  },
  CRITICAL: {
    label: "Critical Risk",
    badgeClass: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400",
    iconClass: "text-red-500",
    Icon: ShieldX,
  },
};

interface RiskBadgeProps {
  risk: RiskScore;
  /** Show expandable factor breakdown. Default false (compact mode for cards). */
  showDetails?: boolean;
}

export function RiskBadge({ risk, showDetails = false }: RiskBadgeProps) {
  const [open, setOpen] = useState(false);
  const cfg = LEVEL_CONFIG[risk.level];
  const { Icon } = cfg;

  return (
    <div>
      <button
        onClick={showDetails ? () => setOpen((v) => !v) : undefined}
        className={`inline-flex items-center gap-1.5 text-xs font-semibold px-2 py-0.5 rounded-full ${cfg.badgeClass} ${showDetails ? "cursor-pointer hover:opacity-80 transition-opacity" : "cursor-default"}`}
        aria-expanded={showDetails ? open : undefined}
        title={showDetails ? "Click to see risk factors" : cfg.label}
      >
        <Icon size={12} className={cfg.iconClass} />
        {cfg.label}
        {risk.total > 0 && (
          <span className="opacity-70">({risk.total})</span>
        )}
        {showDetails && (open ? <ChevronUp size={11} /> : <ChevronDown size={11} />)}
      </button>

      {showDetails && open && risk.factors.length > 0 && (
        <div className="mt-2 border border-[var(--card-border)] rounded-xl overflow-hidden">
          <div className="px-3 py-2 bg-[var(--muted-bg)] border-b border-[var(--card-border)]">
            <p className="text-xs font-semibold text-[var(--foreground)]">
              Risk Factors ({risk.factors.length})
            </p>
          </div>
          <ul className="divide-y divide-[var(--card-border)]">
            {risk.factors.map((factor) => (
              <li key={factor.id} className="px-3 py-2.5 flex items-start gap-2">
                <ShieldAlert size={13} className="text-[var(--muted)] shrink-0 mt-0.5" />
                <div>
                  <p className="text-xs font-medium text-[var(--foreground)]">{factor.label}</p>
                  <p className="text-xs text-[var(--muted)] mt-0.5">{factor.description}</p>
                </div>
                <span className="ml-auto text-xs font-mono text-[var(--muted)] shrink-0">
                  +{factor.score}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {showDetails && open && risk.factors.length === 0 && (
        <p className="mt-2 text-xs text-[var(--muted)]">No risk factors detected.</p>
      )}
    </div>
  );
}
