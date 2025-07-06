// frontend/src/lib/utils/apiUtils.ts
import { loadingStore } from './loadingStore';
import { sessionStore } from '../stores/session';
import { errorStore } from './errorStore';
import { get } from 'svelte/store';

export interface FetchOptions extends RequestInit {
  isFormData?: boolean;
  fetchFn?: typeof fetch;
  timeoutMs?: number;
}

class HttpError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
    this.name = 'HttpError';
  }
}

const CSRF_HEADER = 'X-CSRF-Token';

export async function apiFetch(url: string, options: FetchOptions = {}): Promise<Response> {
  const { isFormData, fetchFn = fetch, timeoutMs = 10000, ...init } = options;
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

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  init.signal = controller.signal;

  loadingStore.start();
  try {
    const res = await fetchFn(url, init);
    if (!res.ok) {
      let message = `HTTP ${res.status}`;
      try {
        const data = await res.json();
        message = (data as any).error || message;
      } catch {
        try {
          message = (await res.text()) || message;
        } catch {
          /* ignore */
        }
      }
      errorStore.show(message);
      throw new HttpError(message, res.status);
    }
    return res;
  } catch (err: any) {
    if (err.name === 'AbortError') {
      err = new Error('Request timed out');
      errorStore.show(err.message);
    } else if (!(err instanceof HttpError)) {
      errorStore.show(err.message || 'Request failed');
    }
    throw err;
  } finally {
    clearTimeout(timer);
    loadingStore.end();
  }
}

export async function getJSON<T>(url: string, options: FetchOptions = {}): Promise<T> {
  const res = await apiFetch(url, { ...options, method: 'GET' });
  return res.json();
}

export async function postJSON<T>(url: string, body: any, options: FetchOptions = {}): Promise<T> {
  const res = await apiFetch(url, {
    ...options,
    method: 'POST',
    body: options.isFormData ? body : JSON.stringify(body)
  });
  return res.json();
}

export async function putJSON<T>(url: string, body: any, options: FetchOptions = {}): Promise<T> {
  const res = await apiFetch(url, {
    ...options,
    method: 'PUT',
    body: options.isFormData ? body : JSON.stringify(body)
  });
  return res.json();
}

export async function deleteJSON<T>(url: string, options: FetchOptions = {}): Promise<T> {
  const res = await apiFetch(url, { ...options, method: 'DELETE' });
  return res.json();
}
