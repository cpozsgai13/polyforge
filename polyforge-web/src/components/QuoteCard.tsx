import { useBestQuote, useLiveQuote } from '../hooks/useQuotes';
import './QuoteCard.css';

interface Props {
  symbol: string;
}

const fmt = (v: number | null | undefined, decimals = 4) =>
  v != null ? v.toFixed(decimals) : '—';

export default function QuoteCard({ symbol }: Props) {
  const { data: book, isError: bookError, refetch: refetchBook, isFetching: bookFetching } = useBestQuote(symbol);
  const { data: live, isError: liveError, isLoading: liveLoading, refetch: refetchLive, isFetching: liveFetching } = useLiveQuote(symbol);

  const marketState = live?.market_state ?? '—';
  console.log(`[QuoteCard] symbol=${symbol} market_state=${marketState} liveLoading=${liveLoading} liveError=${liveError}`);
  const stateClass =
    marketState === 'REGULAR' ? 'state-regular' :
    marketState === 'PRE'     ? 'state-pre' :
    marketState === 'POST'    ? 'state-post' : 'state-closed';

  return (
    <div className="quote-card">
      <div className="quote-card-header">
        <span className="symbol">{symbol}</span>
        <span className={`market-state ${stateClass}`}>
          {liveLoading ? <span className="loading-dot" /> : marketState}
        </span>
      </div>

      <div className="quote-card-live">
        <div className="section-header">
          <span className="book-header">Live Quote</span>
          <button
            className={`refresh-btn${liveFetching ? ' spinning' : ''}`}
            onClick={() => refetchLive()}
            title="Refresh live quote"
            disabled={liveFetching}
          >⟳</button>
        </div>
        <div className="live-row">
          <span className="label">Last</span>
          <span className="value last">{liveError ? '—' : fmt(live?.last_price, 2)}</span>
          <span className="currency">{live?.currency ?? ''}</span>
        </div>
        <div className="live-row">
          <span className="label">Bid (live)</span>
          <span className="value bid">{liveError ? '—' : fmt(live?.bid, 4)}</span>
        </div>
        <div className="live-row">
          <span className="label">Ask (live)</span>
          <span className="value ask">{liveError ? '—' : fmt(live?.ask, 4)}</span>
        </div>
      </div>

      <div className="quote-card-book">
        <div className="section-header">
          <span className="book-header">Order Book</span>
          <button
            className={`refresh-btn${bookFetching ? ' spinning' : ''}`}
            onClick={() => refetchBook()}
            title="Refresh order book"
            disabled={bookFetching}
          >⟳</button>
        </div>
        <div className="book-row">
          <span className="label">Best Bid</span>
          {bookError || !book?.bid
            ? <span className="value empty">—</span>
            : <span className="value bid">{fmt(book.bid.price)} × {book.bid.volume}</span>}
        </div>
        <div className="book-row">
          <span className="label">Best Ask</span>
          {bookError || !book?.ask
            ? <span className="value empty">—</span>
            : <span className="value ask">{fmt(book.ask.price)} × {book.ask.volume}</span>}
        </div>
        {book?.bid && book?.ask && (
          <div className="book-row spread">
            <span className="label">Spread</span>
            <span className="value">{(book.ask.price - book.bid.price).toFixed(4)}</span>
          </div>
        )}
      </div>
    </div>
  );
}
