import axios from 'axios';
import type {
  AckResponse,
  AddOrderRequest,
  AlertEvent,
  BestQuoteResponse,
  CancelOrderRequest,
  CreateAlertRequest,
  LiveQuote,
  PriceAlert,
} from './types';

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

const http = axios.create({
  baseURL: BASE_URL,
  // The server uses a self-signed cert in dev — the browser will block it
  // unless you've accepted it at https://localhost:8443 first.
  headers: { 'Content-Type': 'application/json' },
});

// ── Order book ────────────────────────────────────────────────────────────────

export const getBestQuote = (symbol: string): Promise<BestQuoteResponse> =>
  http.get<BestQuoteResponse>(`/trading/${symbol}/best_quote`).then(r => r.data);

export const getBestBid = (symbol: string): Promise<BestQuoteResponse> =>
  http.get<BestQuoteResponse>(`/trading/${symbol}/best_bid`).then(r => r.data);

export const getBestAsk = (symbol: string): Promise<BestQuoteResponse> =>
  http.get<BestQuoteResponse>(`/trading/${symbol}/best_ask`).then(r => r.data);

export const addOrder = (symbol: string, body: AddOrderRequest): Promise<AckResponse> =>
  http.post<AckResponse>(`/trading/${symbol}/add_order`, body).then(r => r.data);

export const cancelOrder = (symbol: string, body: CancelOrderRequest): Promise<AckResponse> =>
  http.delete<AckResponse>(`/trading/${symbol}/cancel_order`, { data: body }).then(r => r.data);

// ── Market data ───────────────────────────────────────────────────────────────

export const getLiveQuote = (symbol: string): Promise<LiveQuote> =>
  http.get<LiveQuote>(`/market/${symbol}/quote`).then(r => r.data);

// ── Price alerts ──────────────────────────────────────────────────────────────

export const getAlerts = (): Promise<PriceAlert[]> =>
  http.get<PriceAlert[]>('/api/alerts').then(r => r.data);

export const createAlert = (body: CreateAlertRequest): Promise<PriceAlert> =>
  http.post<PriceAlert>('/api/alerts', body).then(r => r.data);

export const deleteAlert = (id: number): Promise<void> =>
  http.delete(`/api/alerts/${id}`).then(() => undefined);

/** Open a native EventSource to the alert SSE stream.
 *  Call the returned `close()` fn to disconnect. */
export function subscribeAlertEvents(
  onEvent: (e: AlertEvent) => void,
  onError?: (e: Event) => void,
): () => void {
  const url = `${BASE_URL}/api/alerts/events`;
  const es  = new EventSource(url);
  es.onmessage = (msg) => {
    try {
      onEvent(JSON.parse(msg.data) as AlertEvent);
    } catch (err) {
      console.warn('[SSE] parse error', err);
    }
  };
  if (onError) es.onerror = onError;
  return () => es.close();
}
