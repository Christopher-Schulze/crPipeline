export interface ReconnectingEventSource {
  getEventSource(): EventSource;
  close(): void;
}

export function createReconnectingEventSource(
  url: string,
  onMessage: (e: MessageEvent) => void,
  retryDelay = 1000,
  onOpen?: () => void,
  onError?: () => void
): ReconnectingEventSource {
  let es: EventSource;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let closed = false;

  const connect = () => {
    if (closed) return;
    es = new EventSource(url);
    es.onmessage = onMessage;
    if (onOpen) {
      es.onopen = onOpen;
    }
    es.onerror = () => {
      if (onError) onError();
      es.close();
      if (!closed) {
        timer = setTimeout(connect, retryDelay);
      }
    };
  };

  connect();

  const close = () => {
    closed = true;
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    es.onerror = null;
    es.onmessage = null;
    if (es.onopen) {
      es.onopen = null;
    }
    es.close();
  };

  const getEventSource = () => es;

  return { getEventSource, close };
}

export interface EventStream {
  close(): void;
}

/**
 * Create an event stream that uses SSE when available and falls back to polling
 * via the provided `pollFn` when SSE is unavailable or reconnecting.
 */
export function createEventStreamWithFallback(
  url: string,
  onMessage: (e: MessageEvent) => void,
  pollFn: () => Promise<void>,
  pollInterval = 10000,
  retryDelay = 1000,
): EventStream {
  let es: ReconnectingEventSource | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const startPolling = () => {
    if (pollTimer) return;
    pollFn();
    pollTimer = setInterval(pollFn, pollInterval);
  };

  const stopPolling = () => {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  };

  if (typeof EventSource === 'undefined') {
    startPolling();
    return { close: stopPolling };
  }

  startPolling();
  es = createReconnectingEventSource(
    url,
    onMessage,
    retryDelay,
    () => stopPolling(),
    () => startPolling()
  );

  return {
    close() {
      es?.close();
      stopPolling();
    }
  };
}
