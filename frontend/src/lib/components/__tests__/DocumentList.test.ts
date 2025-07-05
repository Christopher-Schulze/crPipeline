import { render, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import DocumentList from '../DocumentList.svelte';

vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
  ok: true,
  json: async () => ({
    items: [
      {
        id: '1',
        filename: 'doc1.pdf',
        display_name: 'Doc One',
        is_target: false,
        upload_date: '2023-01-01T00:00:00Z'
      }
    ],
    page: 1,
    total_items: 1,
    per_page: 10,
    total_pages: 1,
    sort_by: 'upload_date',
    sort_order: 'desc'
  })
}) as any));

test('renders documents from api', async () => {
  const { getByText } = render(DocumentList, { props: { orgId: 'org1' } });
  await waitFor(() => {
    expect(getByText('Doc One')).toBeInTheDocument();
  });
});
