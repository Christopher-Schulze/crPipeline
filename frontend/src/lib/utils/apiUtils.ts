// frontend/src/lib/utils/apiUtils.ts

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

  // Add CSRF token for state-changing methods
  const method = options.method?.toUpperCase() || 'GET';
  if (['POST', 'PUT', 'DELETE', 'PATCH'].includes(method)) {
    const token = getCookie('csrf-token'); // Default cookie name for actix-csrf
    if (token) {
      headers.set('X-CSRF-TOKEN', token); // Default header name for actix-csrf
    } else {
      // Only warn if not in SSR context, as document.cookie isn't available there.
      // Backend GET requests (like for initial page load) will set the cookie.
      if (typeof document !== 'undefined') {
        console.warn('CSRF token cookie "csrf-token" not found. State-changing request might fail if this is a client-side navigation/action after initial load.');
      }
    }
  }

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

  options.headers = headers;

  return fetch(url, options);
}
