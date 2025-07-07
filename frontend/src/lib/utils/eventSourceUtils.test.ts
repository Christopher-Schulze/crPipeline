import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createReconnectingEventSource, createEventStreamWithFallback } from './eventSourceUtils';

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
    const onOpen = vi.fn();
    const onError = vi.fn();
    const { getEventSource, close } = createReconnectingEventSource('/test', () => {}, 500, onOpen, onError);

    expect(MockEventSource.instances.length).toBe(1);
    expect(onOpen).toHaveBeenCalledTimes(1);

    const first = getEventSource() as unknown as MockEventSource;
    first.onerror && first.onerror();
    expect(onError).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(500);

    expect(MockEventSource.instances.length).toBe(2);
    expect(onOpen).toHaveBeenCalledTimes(2);

    close();

    const second = getEventSource() as unknown as MockEventSource;
    second.onerror && second.onerror();
    vi.advanceTimersByTime(500);
    expect(onError).toHaveBeenCalledTimes(2);
    expect(MockEventSource.instances.length).toBe(2);
  });
});

describe('createEventStreamWithFallback', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('uses polling when EventSource is undefined', () => {
    vi.stubGlobal('EventSource', undefined as any);
    const pollFn = vi.fn();
    const stream = createEventStreamWithFallback('/poll', () => {}, pollFn, 1000);

    expect(pollFn).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(1000);
    expect(pollFn).toHaveBeenCalledTimes(2);

    stream.close();
    vi.advanceTimersByTime(1000);
    expect(pollFn).toHaveBeenCalledTimes(2);
  });
});
