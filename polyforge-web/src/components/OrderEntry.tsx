import { useState, useEffect } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { addOrder, cancelOrder } from '../api/client';
import { useLiveQuote } from '../hooks/useQuotes';
import type { AddOrderRequest } from '../api/types';
import './OrderEntry.css';

interface Props {
  symbol: string;
}

let nextOrderId = Date.now();

export default function OrderEntry({ symbol }: Props) {
  const qc = useQueryClient();
  const [side, setSide]           = useState<'bid' | 'ask'>('bid');
  const [price, setPrice]         = useState('');
  const [quantity, setQuantity]   = useState('');
  const [orderType, setOrderType] = useState('gfd');
  const [cancelId, setCancelId]   = useState('');
  const [status, setStatus]       = useState<string | null>(null);
  const [priceTouched, setPriceTouched] = useState(false);

  const { data: live } = useLiveQuote(symbol);

  // Pre-fill price from live last_price whenever the symbol changes or the
  // first quote arrives — but don't clobber a value the user has typed.
  useEffect(() => {
    if (!priceTouched && live?.last_price != null) {
      setPrice(live.last_price.toFixed(4));
    }
  }, [symbol, live?.last_price]);

  // Reset touched flag when symbol changes so the new symbol's price seeds in.
  useEffect(() => {
    setPriceTouched(false);
    setPrice('');
    setStatus(null);
  }, [symbol]);

  const addMut = useMutation({
    mutationFn: (body: AddOrderRequest) => addOrder(symbol, body),
    onSuccess: (r) => {
      setStatus(r.success ? `✓ ${r.message}` : `✗ ${r.message}`);
      qc.invalidateQueries({ queryKey: ['bestQuote', symbol] });
      setPrice(''); setQuantity('');
    },
    onError: (e: Error) => setStatus(`✗ ${e.message}`),
  });

  const cancelMut = useMutation({
    mutationFn: (id: number) => cancelOrder(symbol, { order_id: id, instrument_id: 0 }),
    onSuccess: (r) => {
      setStatus(r.success ? `✓ ${r.message}` : `✗ ${r.message}`);
      qc.invalidateQueries({ queryKey: ['bestQuote', symbol] });
      setCancelId('');
    },
    onError: (e: Error) => setStatus(`✗ ${e.message}`),
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const p = parseFloat(price);
    const q = parseInt(quantity, 10);
    if (isNaN(p) || isNaN(q) || q <= 0) {
      setStatus('✗ Invalid price or quantity');
      return;
    }
    addMut.mutate({
      order_id: nextOrderId++,
      price: p,
      quantity: q,
      side,
      order_type: orderType,
      instrument_id: 0,
      timestamp_ns: 0,
    });
  };

  const handleCancel = (e: React.FormEvent) => {
    e.preventDefault();
    const id = parseInt(cancelId, 10);
    if (isNaN(id)) { setStatus('✗ Invalid order ID'); return; }
    cancelMut.mutate(id);
  };

  return (
    <div className="order-entry">
      <div className="oe-title">Order Entry — {symbol}</div>

      <form className="oe-form" onSubmit={handleSubmit}>
        <div className="oe-row">
          <label>Side</label>
          <div className="oe-side-toggle">
            <button type="button" className={side === 'bid' ? 'active bid' : ''} onClick={() => setSide('bid')}>BID</button>
            <button type="button" className={side === 'ask' ? 'active ask' : ''} onClick={() => setSide('ask')}>ASK</button>
          </div>
        </div>
        <div className="oe-row">
          <label>Price</label>
          <input type="number" step="0.0001" placeholder="0.0000" value={price}
            onChange={e => { setPriceTouched(true); setPrice(e.target.value); }} />
        </div>
        <div className="oe-row">
          <label>Qty</label>
          <input type="number" step="1" min="1" placeholder="100" value={quantity} onChange={e => setQuantity(e.target.value)} />
        </div>
        <div className="oe-row">
          <label>Type</label>
          <select value={orderType} onChange={e => setOrderType(e.target.value)}>
            <option value="gfd">GFD</option>
            <option value="ioc">IOC</option>
          </select>
        </div>
        <button type="submit" className={`oe-submit ${side}`} disabled={addMut.isPending}>
          {addMut.isPending ? 'Sending…' : side === 'bid' ? 'Place Bid Order' : 'Place Ask Order'}
        </button>
      </form>

      <form className="oe-cancel-form" onSubmit={handleCancel}>
        <input type="number" placeholder="Order ID to cancel" value={cancelId} onChange={e => setCancelId(e.target.value)} />
        <button type="submit" disabled={cancelMut.isPending}>
          {cancelMut.isPending ? '…' : 'Cancel'}
        </button>
      </form>

      {status && <div className={`oe-status ${status.startsWith('✓') ? 'ok' : 'err'}`}>{status}</div>}
    </div>
  );
}
