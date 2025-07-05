import { writable } from 'svelte/store';

function createLoadingStore() {
  const { subscribe, set, update } = writable(0);
  return {
    subscribe,
    start: () => update(n => n + 1),
    end: () => update(n => Math.max(0, n - 1)),
    reset: () => set(0)
  };
}

export const loadingStore = createLoadingStore();
