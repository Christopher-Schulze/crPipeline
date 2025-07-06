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
