import type { LayoutLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { apiFetch } from '$lib/utils/apiUtils';

export const load: LayoutLoad = async ({ fetch, url }) => {
  // If executed in browser and session is already populated by a previous run (e.g. from server on initial load),
  // we might not need to fetch /api/me again unless we want to re-validate.
  // SvelteKit's fetch is isomorphic.
  // For simplicity, always try to fetch, but a more advanced setup might check existing `parentData` or a store.

  // If we have session data from a server-side +layout.server.ts, it would be in parentData.
  // For now, assume we always fetch client-side or universally if this is the first load function.
  // console.log('Root +layout.ts load function executing. Browser:', browser);

  try {
    const res = await apiFetch('/api/me', { fetchFn: fetch });

    let session = {
      loggedIn: false,
      userId: null as string | null,
      org: null as string | null,
      role: null as string | null,
      csrfToken: import.meta.env.VITE_CSRF_TOKEN ?? null
    };

    if (res.ok) {
      const userData = await res.json();
      session = {
        loggedIn: true,
        userId: userData.user_id,
        org: userData.org_id,
        role: userData.role,
        csrfToken: import.meta.env.VITE_CSRF_TOKEN ?? null
      };
    }

    const path = url.pathname;
    const publicPaths = ['/', '/login', '/register'];

    if (!session.loggedIn && !publicPaths.includes(path)) {
      throw redirect(302, '/login');
    }

    if (path.startsWith('/admin') && session.role !== 'admin') {
      throw redirect(302, '/dashboard');
    }

    if (path.startsWith('/organization') && session.role !== 'org_admin' && session.role !== 'admin') {
      throw redirect(302, '/dashboard');
    }

    return { session };
  } catch (e) {
    return {
      session: {
        loggedIn: false,
        userId: null,
        org: null,
        role: null,
        csrfToken: import.meta.env.VITE_CSRF_TOKEN ?? null
      }
    };
  }
};
