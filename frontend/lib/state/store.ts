import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Product, TrackingEvent } from "../types";
import { isConnected } from "@stellar/freighter-api";

interface SupplyLinkStore {
  products: Product[];
  events: TrackingEvent[];
  walletAddress: string | null;
  xlmBalance: string | null;
  networkMismatch: boolean;
  contractPaused: boolean;
  lastFetched: number | null;
  productPage: number;
  productPageSize: number;
  productTotal: number;
  eventPage: number;
  eventPageSize: number;
  eventTotal: number;
  setWalletAddress: (address: string | null) => void;
  setXlmBalance: (balance: string | null) => void;
  setNetworkMismatch: (mismatch: boolean) => void;
  setContractPaused: (paused: boolean) => void;
  addProduct: (product: Product) => void;
  addEvent: (event: TrackingEvent) => void;
  setProducts: (products: Product[]) => void;
  setEvents: (events: TrackingEvent[]) => void;
  setLastFetched: (ts: number) => void;
  updateProductOwner: (productId: string, newOwner: string) => void;
  validateWalletConnection: () => Promise<void>;
  setProductPage: (page: number) => void;
  setProductPageSize: (size: number) => void;
  setProductTotal: (total: number) => void;
  setEventPage: (page: number) => void;
  setEventPageSize: (size: number) => void;
  setEventTotal: (total: number) => void;
  disconnect: () => void;
}

export const useStore = create<SupplyLinkStore>()(
  persist(
    (set) => ({
      products: [],
      events: [],
      walletAddress: null,
      xlmBalance: null,
      networkMismatch: false,
      contractPaused: false,
      lastFetched: null,
      productPage: 0,
      productPageSize: 20,
      productTotal: 0,
      eventPage: 0,
      eventPageSize: 20,
      eventTotal: 0,
      setWalletAddress: (address) => set({ walletAddress: address }),
      setXlmBalance: (balance) => set({ xlmBalance: balance }),
      setNetworkMismatch: (mismatch) => set({ networkMismatch: mismatch }),
      setContractPaused: (paused) => set({ contractPaused: paused }),
      addProduct: (product) =>
        set((state) => ({ products: [...state.products, product] })),
      addEvent: (event) =>
        set((state) => ({ events: [...state.events, event] })),
      setProducts: (products) => set({ products }),
      setEvents: (events) => set({ events }),
      setLastFetched: (ts) => set({ lastFetched: ts }),
      updateProductOwner: (productId, newOwner) =>
        set((state) => ({
          products: state.products.map((p) =>
            p.id === productId ? { ...p, owner: newOwner } : p
          ),
        })),
      validateWalletConnection: async () => {
        const connected = await isConnected();
        if (!connected) {
          set({ walletAddress: null });
        }
      },
      setProductPage: (page) => set({ productPage: page }),
      setProductPageSize: (size) => set({ productPageSize: size }),
      setProductTotal: (total) => set({ productTotal: total }),
      setEventPage: (page) => set({ eventPage: page }),
      setEventPageSize: (size) => set({ eventPageSize: size }),
      setEventTotal: (total) => set({ eventTotal: total }),
      disconnect: () =>
        set({
          walletAddress: null,
          products: [],
          events: [],
          lastFetched: null,
          productPage: 0,
          eventPage: 0,
        }),
    }),
    {
      name: "supply-link-store",
      partialize: (state) => ({
        walletAddress: state.walletAddress,
      }),
    }
  )
);
