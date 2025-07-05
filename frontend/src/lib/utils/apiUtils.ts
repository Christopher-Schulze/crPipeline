// frontend/src/lib/utils/apiUtils.ts
import { loadingStore } from './loadingStore';
import { sessionStore } from './sessionStore';
import { get } from 'svelte/store';

export interface FetchOptions extends RequestInit {
  isFormData?: boolean;
  fetchFn?: typeof fetch;
}

const CSRF_HEADER = 'X-CSRF-Token';

export async function apiFetch(url: string, options: FetchOptions = {}): Promise<Response> {
  const { isFormData, fetchFn = fetch, ...init } = options;
  const headers = new Headers(init.headers || {});

  if (!isFormData && init.body && typeof init.body === 'string' && !headers.has('Content-Type')) {
    try {
      JSON.parse(init.body);
      headers.set('Content-Type', 'application/json');
    } catch {
      /* body is not JSON */
    }
  }

  init.credentials = 'include';

  const { csrfToken } = get(sessionStore);
  const token = csrfToken || (typeof window !== 'undefined' ? (window as any).CSRF_TOKEN : undefined) || import.meta.env.VITE_CSRF_TOKEN;
  if (token && !headers.has(CSRF_HEADER)) {
    headers.set(CSRF_HEADER, token);
  }

  init.headers = headers;

  loadingStore.start();
  try {
    const res = await fetchFn(url, init);
    if (!res.ok) {
      let message = `HTTP ${res.status}`;
      try {
        const data = await res.json();
        message = data.error || message;
      } catch {
        try {
          message = await res.text() || message;
        } catch {
          /* ignore */
        }
      }
      throw new Error(message);
    }
    return res;
  } finally {
    loadingStore.end();
  }
}
