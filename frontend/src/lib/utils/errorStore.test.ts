import { describe, it, expect, vi } from 'vitest';
import { errorStore } from './errorStore';
import { tick } from 'svelte';

describe('errorStore queue', () => {
  it('shows messages sequentially', async () => {
    vi.useFakeTimers();
    const received: string[] = [];
    const unsub = errorStore.subscribe(items => {
      if (items[0]) {
        received.push(items[0].message);
      }
    });

    errorStore.show('first', 1000);
    errorStore.show('second', 1000);

    await tick();
    expect(received.at(-1)).toBe('first');

    vi.advanceTimersByTime(1000);
    await tick();
    expect(received.at(-1)).toBe('second');

    unsub();
    vi.useRealTimers();
  });
});
