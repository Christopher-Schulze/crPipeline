import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';

export const load: PageLoad = async ({ parent }) => {
  const { session } = await parent();
  if (!session.loggedIn) {
    throw redirect(302, '/login');
  }
  if (session.role !== 'admin') {
    throw redirect(302, '/dashboard');
  }
  return {};
};
