"use client";

import { useState } from "react";
import { AppNavbar } from "./Navbar";
import { Sidebar } from "./Sidebar";
import { AuthGuard } from "./AuthGuard";
import { PausedBanner } from "@/components/wallet/PausedBanner";

export function AppShell({ children }: { children: React.ReactNode }) {
  const [sidebarOpen, setSidebarOpen] = useState(false);

  return (
    <AuthGuard>
      <div className="flex flex-col min-h-screen bg-[var(--background)]">
        <PausedBanner />
        <AppNavbar onMenuClick={() => setSidebarOpen(true)} />
        <div className="flex flex-1">
          <Sidebar open={sidebarOpen} onClose={() => setSidebarOpen(false)} />
          <main className="flex-1 overflow-auto">
            {children}
          </main>
        </div>
      </div>
    </AuthGuard>
  );
}
