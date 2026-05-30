"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Search, Plus, Package, ChevronDown } from "lucide-react";
import * as Select from "@radix-ui/react-select";
import { useStore } from "@/lib/state/store";
import { listProducts } from "@/lib/stellar/client";
import {
  MOCK_PRODUCTS,
  getEventsByProductId,
  getArchivedEventsByProductId,
  getCertificationsByProductId,
} from "@/lib/mock/products";
import { RegisterProductForm } from "@/components/products/RegisterProductForm";
import { RiskBadge } from "@/components/products/RiskBadge";
import ProductQRCode from "@/components/products/ProductQRCode";
import { computeRiskScore } from "@/lib/risk/scorer";
import type { EventType, Product, RiskScore } from "@/lib/types";

const EVENT_TYPES: EventType[] = ["HARVEST", "PROCESSING", "SHIPPING", "RETAIL"];

function ProductSkeleton() {
  return (
    <div className="border border-[var(--card-border)] bg-[var(--card)] rounded-xl p-6 flex flex-col gap-4 animate-pulse">
      <div className="h-5 bg-[var(--muted-bg)] rounded w-3/4" />
      <div className="h-4 bg-[var(--muted-bg)] rounded w-1/2" />
      <div className="h-3 bg-[var(--muted-bg)] rounded w-2/3" />
      <div className="w-40 h-40 bg-[var(--muted-bg)] rounded-lg" />
    </div>
  );
}

function EmptyState({ hasSearch, onRegister }: { hasSearch: boolean; onRegister: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center py-24 text-center gap-4">
      <div className="w-20 h-20 rounded-2xl bg-[var(--muted-bg)] flex items-center justify-center">
        <Package size={36} className="text-[var(--muted)]" />
      </div>
      <h3 className="text-lg font-semibold">
        {hasSearch ? "No products match your search" : "No products yet"}
      </h3>
      <p className="text-sm text-[var(--muted)] max-w-xs">
        {hasSearch
          ? "Try a different name or ID."
          : "Register your first product on-chain to get started."}
      </p>
      {!hasSearch && (
        <button
          onClick={onRegister}
          className="flex items-center gap-2 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors mt-2"
        >
          <Plus size={16} /> Register New Product
        </button>
      )}
    </div>
  );
}

function getRiskForProduct(product: Product): RiskScore {
  return computeRiskScore({
    product,
    activeEvents: getEventsByProductId(product.id),
    archivedEvents: getArchivedEventsByProductId(product.id),
    certifications: getCertificationsByProductId(product.id),
  });
}

export default function ProductsPage() {
  const { products, setProducts } = useStore();
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [eventFilter, setEventFilter] = useState<EventType | "ALL">("ALL");
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      try {
        const { products: onChain } = await listProducts();
        const merged = onChain.length > 0 ? onChain : MOCK_PRODUCTS;
        setProducts(merged);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [setProducts]);

  const filtered = products.filter((p) => {
    const matchesSearch =
      search === "" ||
      p.name.toLowerCase().includes(search.toLowerCase()) ||
      p.id.toLowerCase().includes(search.toLowerCase());
    return matchesSearch;
  });

  return (
    <main className="p-6 md:p-8 max-w-7xl mx-auto">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-8">
        <div>
          <h1 className="text-2xl font-bold text-[var(--foreground)]">Products</h1>
          <p className="text-sm text-[var(--muted)] mt-1">
            {loading ? "Loading…" : `${filtered.length} product${filtered.length !== 1 ? "s" : ""}`}
          </p>
        </div>
        <button
          onClick={() => setModalOpen(true)}
          className="flex items-center gap-2 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors self-start sm:self-auto"
        >
          <Plus size={16} /> Register New Product
        </button>
      </div>

      <div className="flex flex-col sm:flex-row gap-3 mb-8">
        <div className="relative flex-1">
          <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--muted)]" />
          <input
            type="text"
            placeholder="Search by name or ID…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-4 py-2 rounded-lg border border-[var(--card-border)] bg-[var(--card)] text-sm focus:outline-none focus:ring-2 focus:ring-violet-500"
          />
        </div>

        <Select.Root value={eventFilter} onValueChange={(v) => setEventFilter(v as EventType | "ALL")}>
          <Select.Trigger className="flex items-center gap-2 px-4 py-2 rounded-lg border border-[var(--card-border)] bg-[var(--card)] text-sm min-w-[160px] focus:outline-none focus:ring-2 focus:ring-violet-500">
            <Select.Value placeholder="Filter by event" />
            <Select.Icon><ChevronDown size={14} /></Select.Icon>
          </Select.Trigger>
          <Select.Portal>
            <Select.Content className="z-50 bg-[var(--background)] border border-[var(--card-border)] rounded-xl shadow-lg overflow-hidden">
              <Select.Viewport className="p-1">
                <Select.Item value="ALL" className="px-3 py-2 text-sm rounded-lg cursor-pointer hover:bg-[var(--muted-bg)] focus:bg-[var(--muted-bg)] outline-none">
                  <Select.ItemText>All Events</Select.ItemText>
                </Select.Item>
                {EVENT_TYPES.map((t) => (
                  <Select.Item key={t} value={t} className="px-3 py-2 text-sm rounded-lg cursor-pointer hover:bg-[var(--muted-bg)] focus:bg-[var(--muted-bg)] outline-none">
                    <Select.ItemText>{t}</Select.ItemText>
                  </Select.Item>
                ))}
              </Select.Viewport>
            </Select.Content>
          </Select.Portal>
        </Select.Root>
      </div>

      {loading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {Array.from({ length: 6 }).map((_, i) => <ProductSkeleton key={i} />)}
        </div>
      ) : filtered.length === 0 ? (
        <EmptyState hasSearch={search !== ""} onRegister={() => setModalOpen(true)} />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {filtered.map((product) => {
            const risk = getRiskForProduct(product);
            return (
              <Link
                key={product.id}
                href={`/products/${product.id}`}
                className="border border-[var(--card-border)] bg-[var(--card)] rounded-xl p-6 flex flex-col gap-4 shadow-sm hover:shadow-md hover:border-violet-500/40 transition-all"
              >
                <div>
                  <div className="flex items-start justify-between gap-2">
                    <h2 className="text-lg font-semibold text-[var(--foreground)] leading-tight">{product.name}</h2>
                    <span className={`shrink-0 text-xs px-2 py-0.5 rounded-full font-medium ${product.active ? "bg-green-500/10 text-green-500" : "bg-[var(--muted-bg)] text-[var(--muted)]"}`}>
                      {product.active ? "Active" : "Inactive"}
                    </span>
                  </div>
                  <p className="text-sm text-[var(--muted)] mt-1">Origin: {product.origin}</p>
                  <p className="text-xs text-[var(--muted)] mt-1 font-mono truncate">ID: {product.id}</p>
                  <div className="mt-2">
                    <RiskBadge risk={risk} />
                  </div>
                </div>
                <ProductQRCode productId={product.id} size={160} />
              </Link>
            );
          })}
        </div>
      )}

      <RegisterProductForm open={modalOpen} onOpenChange={setModalOpen} />
    </main>
  );
}

const EVENT_TYPES: EventType[] = ["HARVEST", "PROCESSING", "SHIPPING", "RETAIL"];

function ProductSkeleton() {
  return (
    <div className="border border-[var(--card-border)] bg-[var(--card)] rounded-xl p-6 flex flex-col gap-4 animate-pulse">
      <div className="h-5 bg-[var(--muted-bg)] rounded w-3/4" />
      <div className="h-4 bg-[var(--muted-bg)] rounded w-1/2" />
      <div className="h-3 bg-[var(--muted-bg)] rounded w-2/3" />
      <div className="w-40 h-40 bg-[var(--muted-bg)] rounded-lg" />
    </div>
  );
}

function EmptyState({ hasSearch, onRegister }: { hasSearch: boolean; onRegister: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center py-24 text-center gap-4">
      <div className="w-20 h-20 rounded-2xl bg-[var(--muted-bg)] flex items-center justify-center">
        <Package size={36} className="text-[var(--muted)]" />
      </div>
      <h3 className="text-lg font-semibold">
        {hasSearch ? "No products match your search" : "No products yet"}
      </h3>
      <p className="text-sm text-[var(--muted)] max-w-xs">
        {hasSearch
          ? "Try a different name or ID."
          : "Register your first product on-chain to get started."}
      </p>
      {!hasSearch && (
        <button
          onClick={onRegister}
          className="flex items-center gap-2 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors mt-2"
        >
          <Plus size={16} /> Register New Product
        </button>
      )}
    </div>
  );
}

export default function ProductsPage() {
  const { products, setProducts } = useStore();
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [eventFilter, setEventFilter] = useState<EventType | "ALL">("ALL");
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      try {
        const { products: onChain } = await listProducts();
        const merged = onChain.length > 0 ? onChain : MOCK_PRODUCTS;
        setProducts(merged);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [setProducts]);

  const filtered = products.filter((p) => {
    const matchesSearch =
      search === "" ||
      p.name.toLowerCase().includes(search.toLowerCase()) ||
      p.id.toLowerCase().includes(search.toLowerCase());
    return matchesSearch;
  });

  return (
    <main className="p-6 md:p-8 max-w-7xl mx-auto">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-8">
        <div>
          <h1 className="text-2xl font-bold text-[var(--foreground)]">Products</h1>
          <p className="text-sm text-[var(--muted)] mt-1">
            {loading ? "Loading…" : `${filtered.length} product${filtered.length !== 1 ? "s" : ""}`}
          </p>
        </div>
        <button
          onClick={() => setModalOpen(true)}
          className="flex items-center gap-2 px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg text-sm font-medium transition-colors self-start sm:self-auto"
        >
          <Plus size={16} /> Register New Product
        </button>
      </div>

      <div className="flex flex-col sm:flex-row gap-3 mb-8">
        <div className="relative flex-1">
          <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--muted)]" />
          <input
            type="text"
            placeholder="Search by name or ID…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-4 py-2 rounded-lg border border-[var(--card-border)] bg-[var(--card)] text-sm focus:outline-none focus:ring-2 focus:ring-violet-500"
          />
        </div>

        <Select.Root value={eventFilter} onValueChange={(v) => setEventFilter(v as EventType | "ALL")}>
          <Select.Trigger className="flex items-center gap-2 px-4 py-2 rounded-lg border border-[var(--card-border)] bg-[var(--card)] text-sm min-w-[160px] focus:outline-none focus:ring-2 focus:ring-violet-500">
            <Select.Value placeholder="Filter by event" />
            <Select.Icon><ChevronDown size={14} /></Select.Icon>
          </Select.Trigger>
          <Select.Portal>
            <Select.Content className="z-50 bg-[var(--background)] border border-[var(--card-border)] rounded-xl shadow-lg overflow-hidden">
              <Select.Viewport className="p-1">
                <Select.Item value="ALL" className="px-3 py-2 text-sm rounded-lg cursor-pointer hover:bg-[var(--muted-bg)] focus:bg-[var(--muted-bg)] outline-none">
                  <Select.ItemText>All Events</Select.ItemText>
                </Select.Item>
                {EVENT_TYPES.map((t) => (
                  <Select.Item key={t} value={t} className="px-3 py-2 text-sm rounded-lg cursor-pointer hover:bg-[var(--muted-bg)] focus:bg-[var(--muted-bg)] outline-none">
                    <Select.ItemText>{t}</Select.ItemText>
                  </Select.Item>
                ))}
              </Select.Viewport>
            </Select.Content>
          </Select.Portal>
        </Select.Root>
      </div>

      {loading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {Array.from({ length: 6 }).map((_, i) => <ProductSkeleton key={i} />)}
        </div>
      ) : filtered.length === 0 ? (
        <EmptyState hasSearch={search !== ""} onRegister={() => setModalOpen(true)} />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {filtered.map((product) => (
            <Link
              key={product.id}
              href={`/products/${product.id}`}
              className="border border-[var(--card-border)] bg-[var(--card)] rounded-xl p-6 flex flex-col gap-4 shadow-sm hover:shadow-md hover:border-violet-500/40 transition-all"
            >
              <div>
                <div className="flex items-start justify-between gap-2">
                  <h2 className="text-lg font-semibold text-[var(--foreground)] leading-tight">{product.name}</h2>
                  <span className={`shrink-0 text-xs px-2 py-0.5 rounded-full font-medium ${product.active ? "bg-green-500/10 text-green-500" : "bg-[var(--muted-bg)] text-[var(--muted)]"}`}>
                    {product.active ? "Active" : "Inactive"}
                  </span>
                </div>
                <p className="text-sm text-[var(--muted)] mt-1">Origin: {product.origin}</p>
                <p className="text-xs text-[var(--muted)] mt-1 font-mono truncate">ID: {product.id}</p>
              </div>
              <ProductQRCode productId={product.id} size={160} />
            </Link>
          ))}
        </div>
      )}

      <RegisterProductForm open={modalOpen} onOpenChange={setModalOpen} />
    </main>
  );
}
