export type EventType = 'HARVEST' | 'PROCESSING' | 'SHIPPING' | 'RETAIL';
export type ProductStatus = 'active' | 'inactive';

export interface TemplateStage {
  label: string;
  eventType: EventType;
}

export interface OwnershipRecord {
  owner: string;
  transferredAt: number;
}

export interface Product {
  id: string;
  name: string;
  origin: string;
  owner: string;
  timestamp: number;
  active: boolean;
  status?: ProductStatus;
  authorizedActors: string[];
  ownershipHistory?: OwnershipRecord[];
  /** Number of signatures required for events (0 or 1 = immediate, >1 = multi-sig) */
  requiredSignatures?: number;
  /** true while an on-chain transaction is in-flight (#49) */
  pending?: boolean;
  /** Off-chain image URL stored in product metadata (#112) */
  imageUrl?: string;
}

export interface TrackingEvent {
  productId: string;
  location: string;
  actor: string;
  timestamp: number;
  eventType: EventType;
  metadata: string;
  /** true while an on-chain transaction is in-flight (#49) */
  pending?: boolean;
}

export interface PendingEvent {
  pendingEventId: number;
  productId: string;
  event: TrackingEvent;
  approvals: string[];
  requiredSignatures: number;
  createdAt: number;
  expiration?: number;
}

export type NotificationType =
  | 'TRACKING_EVENT'
  | 'APPROVAL_PENDING'
  | 'APPROVAL_FINALIZED'
  | 'APPROVAL_REJECTED'
  | 'OWNERSHIP_CHANGED'
  | 'PRODUCT_RECALLED'
  | 'CONTRACT_ERROR';

export interface Notification {
  id: string;
  productId: string;
  productName: string;
  eventType: EventType;
  location: string;
  actor: string;
  timestamp: number;
  read: boolean;
  notificationType: NotificationType;
  message?: string;
}

export interface TransactionResult {
  hash: string;
  status: 'success' | 'failed' | 'pending';
  fee: string;
  timestamp: number;
}

export interface ContractError {
  code: number;
  message: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  offset: number;
  limit: number;
}

export interface Rating {
  id: string;
  productId: string;
  walletAddress: string;
  stars: number;
  comment: string | null;
  timestamp: number;
}
