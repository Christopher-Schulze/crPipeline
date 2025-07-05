// frontend/src/lib/utils/apiUtils.ts
import { loadingStore } from './loadingStore';

const CSRF_HEADER = 'X-CSRF-Token';
const csrfToken =
  typeof window !== 'undefined'
    ? (window as any).CSRF_TOKEN || import.meta.env.VITE_CSRF_TOKEN
    : import.meta.env.VITE_CSRF_TOKEN;

// Function to get a cookie by name
function getCookie(name: string): string | null {
  if (typeof document === 'undefined') {
    // Running in SSR or worker, document is not available
    return null;
  }
  const nameEQ = name + "=";
  const ca = document.cookie.split(';');
  for(let i = 0; i < ca.length; i++) {
    let c = ca[i];
    while (c.charAt(0) == ' ') c = c.substring(1, c.length);
    if (c.indexOf(nameEQ) == 0) return c.substring(nameEQ.length, c.length);
  }
  return null;
}

export interface FetchOptions extends RequestInit {
  isFormData?: boolean; // Special flag if body is FormData
}

export async function apiFetch(url: string, options: FetchOptions = {}): Promise<Response> {
  const headers = new Headers(options.headers || {});


  // Set Content-Type for JSON unless it's FormData or already set
  if (!options.isFormData && options.body && typeof options.body === 'string' && !headers.has('Content-Type')) {
      try {
         JSON.parse(options.body); // Check if body is valid JSON string
         headers.set('Content-Type', 'application/json');
      } catch (e) {
         // Not a JSON string, or Content-Type was already set by caller (e.g. for other types)
         // console.debug("apiFetch: Body provided but not setting Content-Type to JSON. Either not JSON or Content-Type already set.", e);
      }
  }

  // Ensure credentials (cookies) are sent for same-origin and cross-origin requests if CORS allows
  options.credentials = 'include';

  if (csrfToken && !headers.has(CSRF_HEADER)) {
    headers.set(CSRF_HEADER, csrfToken as string);
  }

  options.headers = headers;

  loadingStore.start();
  try {
    return await fetch(url, options);
  } finally {
    loadingStore.end();
  }
}
