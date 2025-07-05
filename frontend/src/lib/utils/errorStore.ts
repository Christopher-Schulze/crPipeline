import { writable } from 'svelte/store';

export interface ErrorItem {
  id: number;
  message: string;
}

function createErrorStore() {
  const { subscribe, update } = writable<ErrorItem[]>([]);

  function remove(id: number) {
    update((items) => items.filter((item) => item.id !== id));
  }

  function show(message: string, timeout = 5000) {
    const id = Date.now() + Math.random();
    update((items) => [...items, { id, message }]);
    if (timeout > 0) {
      setTimeout(() => remove(id), timeout);
    }
  }

  return {
    subscribe,
    show,
    remove
  };
}

export const errorStore = createErrorStore();
