import { render } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';

vi.mock('$app/stores', () => ({
  page: {
    subscribe: (run: any) => { run({ data: { session: { userId: 'u1' } } }); return () => {}; }
  }
}));

vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
  ok: true,
  json: async () => ({ items: [], total_items: 0, page: 1, per_page: 10, total_pages: 1 })
})) as any);

test('renders search field', async () => {
  const { default: UserManagementView } = await import('../UserManagementView.svelte');
  const { getByPlaceholderText } = render(UserManagementView, { props: { orgId: 'o1', orgName: 'Org' } });
  expect(getByPlaceholderText('Search by email...')).toBeInTheDocument();
});
