import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getAlerts, createAlert, deleteAlert } from '../api/client';
import type { PriceAlert } from '../api/types';
import './AlertPanel.css';

interface Props {
  symbols: string[];
  /** Pre-select a symbol when the panel opens from a quote card. */
  defaultSymbol?: string;
}

const fmt = (v: number) => v.toLocaleString(undefined, { maximumFractionDigits: 6 });

const fmtTriggeredAt = (iso: string | null): string => {
  if (!iso) return '';
  const d = new Date(iso);
  return d.toLocaleString(undefined, {
    month: 'short', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
  });
};

export default function AlertPanel({ symbols, defaultSymbol }: Props) {
  const qc = useQueryClient();

  // ── Form state ─────────────────────────────────────────────────────────────
  const [symbol,    setSymbol]    = useState(defaultSymbol ?? symbols[0] ?? '');
  const [threshold, setThreshold] = useState('');
  const [direction, setDirection] = useState<'above' | 'below'>('above');
  const [label,     setLabel]     = useState('');
  const [formErr,   setFormErr]   = useState<string | null>(null);

  // ── Data ───────────────────────────────────────────────────────────────────
  const { data: alerts = [], isLoading } = useQuery({
    queryKey: ['alerts'],
    queryFn:  getAlerts,
    refetchInterval: 10_000,
  });

  const addMut = useMutation({
    mutationFn: createAlert,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['alerts'] });
      setThreshold('');
      setLabel('');
      setFormErr(null);
    },
    onError: (e: unknown) => {
      const msg = e instanceof Error ? e.message : 'Unknown error';
      setFormErr(msg);
    },
  });

  const delMut = useMutation({
    mutationFn: deleteAlert,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['alerts'] }),
  });

  // ── Submit ─────────────────────────────────────────────────────────────────
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const t = parseFloat(threshold);
    if (isNaN(t) || t <= 0) {
      setFormErr('Enter a valid positive price threshold.');
      return;
    }
    setFormErr(null);
    addMut.mutate({
      symbol,
      threshold: t,
      direction,
      label: label.trim() || undefined,
    });
  };

  return (
    <div className="alert-panel">
      <div className="alert-panel-header">
        <span className="alert-panel-title">⚑ Price Alerts</span>
        <span className="alert-count">{alerts.filter((a: PriceAlert) => !a.triggered).length} active</span>
      </div>

      {/* ── Create form ───────────────────────────────────────────────────── */}
      <form className="alert-form" onSubmit={handleSubmit}>
        <div className="alert-form-row">
          <select
            className="alert-select"
            value={symbol}
            onChange={e => setSymbol(e.target.value)}
          >
            {symbols.map(s => <option key={s} value={s}>{s}</option>)}
          </select>

          <div className="direction-toggle">
            <button
              type="button"
              className={`dir-btn ${direction === 'above' ? 'active above' : ''}`}
              onClick={() => setDirection('above')}
            >↑ Above</button>
            <button
              type="button"
              className={`dir-btn ${direction === 'below' ? 'active below' : ''}`}
              onClick={() => setDirection('below')}
            >↓ Below</button>
          </div>
        </div>

        <div className="alert-form-row">
          <input
            className="alert-input"
            type="number"
            step="any"
            min="0"
            placeholder="Price threshold"
            value={threshold}
            onChange={e => setThreshold(e.target.value)}
            required
          />
          <input
            className="alert-input label-input"
            type="text"
            placeholder="Label (optional)"
            value={label}
            onChange={e => setLabel(e.target.value)}
            maxLength={60}
          />
        </div>

        {formErr && <div className="alert-form-err">{formErr}</div>}

        <button
          className="alert-submit"
          type="submit"
          disabled={addMut.isPending}
        >
          {addMut.isPending ? 'Adding…' : 'Add Alert'}
        </button>
      </form>

      {/* ── Active alerts ─────────────────────────────────────────────────── */}
      <div className="alert-list">
        {isLoading && <div className="alert-empty">Loading…</div>}
        {!isLoading && alerts.filter((a: PriceAlert) => !a.triggered).length === 0 && (
          <div className="alert-empty">No active alerts</div>
        )}
        {alerts.filter((a: PriceAlert) => !a.triggered).map((a: PriceAlert) => (
          <div key={a.id} className="alert-row">
            <span className="alert-symbol">{a.symbol}</span>
            <span className={`alert-dir ${a.direction}`}>
              {a.direction === 'above' ? '↑' : '↓'}
            </span>
            <span className="alert-threshold">{fmt(a.threshold)}</span>
            {a.label && <span className="alert-label">{a.label}</span>}
            <button
              className="alert-del"
              title="Remove alert"
              onClick={() => delMut.mutate(a.id)}
              disabled={delMut.isPending}
            >✕</button>
          </div>
        ))}
      </div>

      {/* ── Triggered alerts ─────────────────────────────────────────────── */}
      {alerts.some((a: PriceAlert) => a.triggered) && (
        <div className="alert-triggered-section">
          <span className="alert-triggered-header">⚡ Fired</span>
          <div className="alert-list">
            {alerts.filter((a: PriceAlert) => a.triggered).map((a: PriceAlert) => (
              <div key={a.id} className="alert-row alert-row--fired">
                <div className="alert-fired-main">
                  <span className="alert-symbol">{a.symbol}</span>
                  <span className={`alert-dir ${a.direction}`}>
                    {a.direction === 'above' ? '↑' : '↓'}
                  </span>
                  <span className="alert-threshold">{fmt(a.threshold)}</span>
                  {a.label && <span className="alert-label">{a.label}</span>}
                  <span
                    className="alert-triggered-badge"
                    title={a.triggered_at ? `Triggered at ${fmtTriggeredAt(a.triggered_at)}` : 'Triggered'}
                  >TRIGGERED</span>
                  <button
                    className="alert-del"
                    title="Dismiss"
                    onClick={() => delMut.mutate(a.id)}
                    disabled={delMut.isPending}
                  >✕</button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
