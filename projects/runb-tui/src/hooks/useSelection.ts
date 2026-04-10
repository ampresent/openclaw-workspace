import { useState, useCallback, useEffect } from 'react';

export interface SelectionState<T> {
  index: number;
  items: T[];
  selected: T | undefined;
  up: () => void;
  down: () => void;
  setIndex: (i: number) => void;
  refresh: () => void;
}

export function useSelection<T>(
  fetchItems: () => T[],
  refreshInterval: number = 2000
): SelectionState<T> {
  const [items, setItems] = useState<T[]>(() => fetchItems());
  const [index, setIndex] = useState(0);

  const refresh = useCallback(() => {
    const newItems = fetchItems();
    setItems(newItems);
    setIndex(prev => Math.min(prev, Math.max(0, newItems.length - 1)));
  }, [fetchItems]);

  useEffect(() => {
    const timer = setInterval(refresh, refreshInterval);
    return () => clearInterval(timer);
  }, [refresh, refreshInterval]);

  const up = useCallback(() => {
    setIndex(prev => Math.max(0, prev - 1));
  }, []);

  const down = useCallback(() => {
    setIndex(prev => Math.min(items.length - 1, prev + 1));
  }, [items.length]);

  return {
    index,
    items,
    selected: items[index],
    up,
    down,
    setIndex,
    refresh,
  };
}

export interface TabState {
  tab: number;
  nextTab: () => void;
  prevTab: () => void;
  setTab: (t: number) => void;
}

export function useTabs(count: number): TabState {
  const [tab, setTab] = useState(0);

  return {
    tab,
    nextTab: () => setTab(t => (t + 1) % count),
    prevTab: () => setTab(t => (t - 1 + count) % count),
    setTab,
  };
}
