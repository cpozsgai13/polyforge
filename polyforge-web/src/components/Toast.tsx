import { useEffect, useState, useCallback } from 'react';
import type { AlertEvent } from '../api/types';
import './Toast.css';

interface ToastItem {
  id:    number;
  event: AlertEvent;
}

let _nextId = 0;

interface Props {
  events: AlertEvent[];
}

export default function ToastStack({ events }: Props) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const dismiss = useCallback((id: number) => {
    setToasts(prev => prev.filter(t => t.id !== id));
  }, []);

  // Push a new toast whenever a new event arrives.
  useEffect(() => {
    if (events.length === 0) return;
    const event = events[events.length - 1];
    const id    = ++_nextId;
    setToasts(prev => [...prev, { id, event }]);

    // Auto-dismiss after 8 s.
    const timer = setTimeout(() => dismiss(id), 8000);
    return () => clearTimeout(timer);
  }, [events, dismiss]);

  if (toasts.length === 0) return null;

  return (
    <div className="toast-stack">
      {toasts.map(({ id, event }) => (
        <div key={id} className={`toast toast-alert`}>
          <div className="toast-head">
            <span className="toast-icon">⚑</span>
            <span className="toast-symbol">{event.symbol}</span>
            <span className={`toast-dir ${event.direction}`}>
              {event.direction === 'above' ? '↑ above' : '↓ below'}
            </span>
            <button className="toast-close" onClick={() => dismiss(id)}>✕</button>
          </div>
          <div className="toast-body">
            Price <strong>{event.price.toLocaleString(undefined, { maximumFractionDigits: 6 })}</strong>
            {' '}crossed threshold{' '}
            <strong>{event.threshold.toLocaleString(undefined, { maximumFractionDigits: 6 })}</strong>
            {event.label && <span className="toast-label"> — {event.label}</span>}
          </div>
        </div>
      ))}
    </div>
  );
}
