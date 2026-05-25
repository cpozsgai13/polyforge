import { useQuery } from '@tanstack/react-query';
import { getBestQuote, getLiveQuote } from '../api/client';

export const useBestQuote = (symbol: string) =>
  useQuery({
    queryKey: ['bestQuote', symbol],
    queryFn: () => getBestQuote(symbol),
    refetchInterval: 2000,
    retry: false,
  });

export const useLiveQuote = (symbol: string) =>
  useQuery({
    queryKey: ['liveQuote', symbol],
    queryFn: () => getLiveQuote(symbol),
    refetchInterval: 5000,
    retry: false,
  });
