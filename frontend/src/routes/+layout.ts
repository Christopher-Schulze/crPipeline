import type { LayoutLoad } from './$types';
import { browser } from '$app/environment';

export const load: LayoutLoad = async ({ fetch, data: parentData }) => {
  // If executed in browser and session is already populated by a previous run (e.g. from server on initial load),
  // we might not need to fetch /api/me again unless we want to re-validate.
  // SvelteKit's fetch is isomorphic.
  // For simplicity, always try to fetch, but a more advanced setup might check existing `parentData` or a store.

  // If we have session data from a server-side +layout.server.ts, it would be in parentData.
  // For now, assume we always fetch client-side or universally if this is the first load function.
  // console.log('Root +layout.ts load function executing. Browser:', browser);

  try {
    // Use SvelteKit's fetch for universal loading (works on server and client)
    // It automatically handles credentials (cookies) for same-origin requests.
    const res = await fetch('/api/me');

    if (res.ok) {
      const userData = await res.json();
      // console.log('Root +layout.ts: /api/me success', userData);
      return {
        session: {
          loggedIn: true,
          userId: userData.user_id,
          org: userData.org_id,
          role: userData.role,
          // You could also fetch initial accent_color here if OrgSettings are available via /api/me or another call
        }
      };
    } else {
      // console.log('Root +layout.ts: /api/me returned not ok', res.status);
      // This means user is not authenticated or session expired
      return {
        session: {
          loggedIn: false,
          userId: null,
          org: null,
          role: null,
        }
      };
    }
  } catch (e) {
    // console.error("Root +layout.ts: Failed to fetch session for root layout:", e);
    // Network error or other issue
    return {
      session: {
        loggedIn: false,
        userId: null,
        org: null,
        role: null,
      }
    };
  }
};
