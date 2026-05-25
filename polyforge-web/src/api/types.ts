// ── DTOs — mirror the server-side Rust structs ────────────────────────────────

export interface QuoteLevel {
  price: number;
  volume: number;
}

export interface BestQuoteResponse {
  symbol: string;
  bid: QuoteLevel | null;
  ask: QuoteLevel | null;
}

export interface LiveQuote {
  symbol: string;
  bid: number | null;
  ask: number | null;
  last_price: number | null;
  currency: string | null;
  market_state: string | null;
}

export interface AckResponse {
  success: boolean;
  message: string;
}

export interface AddOrderRequest {
  order_id: number;
  price: number;
  quantity: number;
  side: 'bid' | 'ask';
  order_type: string;
  instrument_id: number;
  timestamp_ns: number;
}

export interface CancelOrderRequest {
  order_id: number;
  instrument_id: number;
}

// ── Price Alerts ──────────────────────────────────────────────────────────────

export interface PriceAlert {
  id: number;
  symbol: string;
  threshold: number;
  direction: 'above' | 'below';
  label: string | null;
  triggered: boolean;
  /** ISO-8601 UTC string when the alert fired, null if not yet triggered. */
  triggered_at: string | null;
}

export interface CreateAlertRequest {
  symbol: string;
  threshold: number;
  direction: 'above' | 'below';
  label?: string;
}

/** Fired over SSE when an alert crosses its threshold. */
export interface AlertEvent {
  alert_id: number;
  symbol: string;
  threshold: number;
  direction: string;
  label: string | null;
  price: number;
}

