# polyforge-web

React 19 + TypeScript + Vite UI. Not part of the Cargo workspace.

- Commands: `npm run dev`, `npm run build` (`tsc -b && vite build`), `npm run lint`.
- Dev server proxies `/trading`, `/market`, `/api` to `https://localhost:8443` (`secure: false`) in [vite.config.ts](vite.config.ts). Run the Rust server with `PORT=8443`. New top-level API paths need a new proxy entry.
- All HTTP goes through [src/api/client.ts](src/api/client.ts) (axios, `VITE_API_BASE_URL`). DTOs in [src/api/types.ts](src/api/types.ts) **mirror the server's Rust structs by hand**; keep them in sync.
- Data fetching uses TanStack Query. Polling hooks are in `src/hooks/useQuotes.ts` (best quote every 2 s, live quote every 5 s, `retry: false`). After mutations, invalidate keys like `['bestQuote', symbol]` and `['alerts']`.
- Alerts: server-sent events via `subscribeAlertEvents` (native `EventSource`), surfaced as toasts in `App.tsx`.
- Symbols come from `VITE_SYMBOLS` (default `AAPL,MSFT,NVDA,BTC-USD,ETH-USD,SOL-USD`); a `-` means crypto (Kraken), otherwise equity (Yahoo).
- Components live in `src/components/` with a co-located `.css` file per component.
