import { writable } from 'svelte/store';

export interface ErrorItem {
  id: number;
  message: string;
}

function createErrorStore() {
  const { subscribe, update } = writable<ErrorItem[]>([]);
  // Queue holds messages waiting to be displayed
  const queue: { message: string; timeout: number }[] = [];

  function processQueue() {
    // Only show next item when none is currently visible
    update((items) => {
      if (items.length === 0 && queue.length > 0) {
        const { message, timeout } = queue.shift()!;
        const id = Date.now() + Math.random();
        if (timeout > 0) {
          setTimeout(() => remove(id), timeout);
        }
        return [{ id, message }];
      }
      return items;
    });
  }

  function remove(id: number) {
    update((items) => items.filter((item) => item.id !== id));
    processQueue();
  }

  function show(message: string, timeout = 5000) {
    queue.push({ message, timeout });
    processQueue();
  }

  return {
    subscribe,
    show,
    remove
  };
}

export const errorStore = createErrorStore();
