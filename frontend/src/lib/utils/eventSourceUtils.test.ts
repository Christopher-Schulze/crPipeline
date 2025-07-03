import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createReconnectingEventSource } from './eventSourceUtils';

class MockEventSource {
  static instances: MockEventSource[] = [];
  onerror: (() => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  constructor(public url: string) {
    MockEventSource.instances.push(this);
  }
  close = vi.fn();
}

describe('createReconnectingEventSource', () => {
  vi.stubGlobal('EventSource', MockEventSource as any);

  beforeEach(() => {
    MockEventSource.instances.length = 0;
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('reconnects after error and stops after close', () => {
    const { getEventSource, close } = createReconnectingEventSource('/test', () => {}, 500);

    expect(MockEventSource.instances.length).toBe(1);

    const first = getEventSource() as unknown as MockEventSource;
    first.onerror && first.onerror();

    vi.advanceTimersByTime(500);

    expect(MockEventSource.instances.length).toBe(2);

    close();

    const second = getEventSource() as unknown as MockEventSource;
    second.onerror && second.onerror();
    vi.advanceTimersByTime(500);

    expect(MockEventSource.instances.length).toBe(2);
  });
});
