import { useState, useEffect } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import QuoteCard from './components/QuoteCard';
import OrderEntry from './components/OrderEntry';
import AlertPanel from './components/AlertPanel';
import ToastStack from './components/Toast';
import { subscribeAlertEvents } from './api/client';
import type { AlertEvent } from './api/types';
import './App.css';

const ALL_SYMBOLS = (import.meta.env.VITE_SYMBOLS ?? 'AAPL,MSFT,NVDA,BTC-USD,ETH-USD,SOL-USD')
  .split(',')
  .map((s: string) => s.trim().toUpperCase())
  .filter(Boolean);

const EQUITY_SYMBOLS = ALL_SYMBOLS.filter((s: string) => !s.includes('-'));
const CRYPTO_SYMBOLS  = ALL_SYMBOLS.filter((s: string) => s.includes('-'));

const TIMEZONES = [
  { label: 'New York',  value: 'America/New_York' },
  { label: 'Chicago',   value: 'America/Chicago' },
  { label: 'Denver',    value: 'America/Denver' },
  { label: 'Los Angeles', value: 'America/Los_Angeles' },
  { label: 'London',    value: 'Europe/London' },
  { label: 'Frankfurt', value: 'Europe/Berlin' },
  { label: 'Zürich',    value: 'Europe/Zurich' },
  { label: 'Dubai',     value: 'Asia/Dubai' },
  { label: 'Singapore', value: 'Asia/Singapore' },
  { label: 'Tokyo',     value: 'Asia/Tokyo' },
  { label: 'Sydney',    value: 'Australia/Sydney' },
  { label: 'UTC',       value: 'UTC' },
];

function useClock(tz: string) {
  const [dt, setDt] = useState({ date: '', time: '' });
  useEffect(() => {
    const tick = () => {
      const now = new Date();
      setDt({
        date: now.toLocaleDateString('en-US', {
          timeZone: tz, weekday: 'short', month: 'short', day: '2-digit', year: 'numeric',
        }),
        time: now.toLocaleTimeString('en-US', {
          timeZone: tz, hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
        }),
      });
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [tz]);
  return dt;
}

const queryClient = new QueryClient();

export default function App() {
  const [activeSymbol, setActiveSymbol] = useState<string>(ALL_SYMBOLS[0] ?? 'AAPL');
  const [section, setSection] = useState<'equities' | 'crypto'>('equities');
  const [timezone, setTimezone] = useState('America/New_York');
  const { date, time } = useClock(timezone);

  // ── SSE: price alert events ────────────────────────────────────────────────
  const [firedEvents, setFiredEvents] = useState<AlertEvent[]>([]);
  useEffect(() => {
    const unsub = subscribeAlertEvents(
      (e) => setFiredEvents(prev => [...prev, e]),
    );
    return unsub;
  }, []);

  const sectionSymbols = section === 'equities' ? EQUITY_SYMBOLS : CRYPTO_SYMBOLS;

  return (
    <QueryClientProvider client={queryClient}>
      <div className="app">
        <header className="app-header">
          <span className="app-logo">⬡ Polyforge</span>

          <div className="header-nav">
            <div className="section-tabs">
              <button
                className={`section-tab ${section === 'equities' ? 'active' : ''}`}
                onClick={() => { setSection('equities'); setActiveSymbol(EQUITY_SYMBOLS[0] ?? ''); }}
              >Equities</button>
              <button
                className={`section-tab ${section === 'crypto' ? 'active' : ''}`}
                onClick={() => { setSection('crypto'); setActiveSymbol(CRYPTO_SYMBOLS[0] ?? ''); }}
              >Crypto</button>
              <span className="quote-source-sep">|</span>
              <select className="quote-source-select" disabled>
                {section === 'equities'
                  ? <option>Yahoo Finance</option>
                  : <option>Kraken</option>
                }
              </select>
            </div>
            <nav className="symbol-tabs">
              {sectionSymbols.map((sym: string) => (
                <button
                  key={sym}
                  className={`tab ${sym === activeSymbol ? 'active' : ''}`}
                  onClick={() => setActiveSymbol(sym)}
                >
                  {sym}
                </button>
              ))}
            </nav>
          </div>

          <div className="tz-widget">
            <select
              className="tz-select"
              value={timezone}
              onChange={e => setTimezone(e.target.value)}
            >
              {TIMEZONES.map(tz => (
                <option key={tz.value} value={tz.value}>{tz.label}</option>
              ))}
            </select>
            <div className="tz-datetime">
              <span className="tz-date">{date}</span>
              <span className="tz-clock">{time}</span>
            </div>
          </div>
        </header>

        <main className="app-main">
          <div className="quotes-row">
            {sectionSymbols.map((sym: string) => (
              <QuoteCard key={sym} symbol={sym} />
            ))}
          </div>

          <div className="trading-panel">
            <OrderEntry symbol={activeSymbol} />
            <AlertPanel symbols={ALL_SYMBOLS} defaultSymbol={activeSymbol} />
          </div>
        </main>

        <ToastStack events={firedEvents} />
      </div>
    </QueryClientProvider>
  );
}
