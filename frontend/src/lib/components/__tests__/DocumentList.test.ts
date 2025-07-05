import { render, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import DocumentList from '../DocumentList.svelte';

const docs = [{
  id: '1',
  filename: 'doc1.pdf',
  display_name: 'Doc One',
  is_target: false,
  upload_date: '2023-01-01T00:00:00Z'
}];

const fetchMock = vi.fn((url: string, options?: any) => {
  if (options && options.method === 'DELETE') {
    docs.pop();
    return Promise.resolve({ ok: true, text: async () => '' });
  }
  return Promise.resolve({
    ok: true,
    json: async () => ({
      items: docs,
      page: 1,
      total_items: docs.length,
      per_page: 10,
      total_pages: 1,
      sort_by: 'upload_date',
      sort_order: 'desc'
    })
  });
}) as any;

vi.stubGlobal('fetch', fetchMock);

test('renders documents from api', async () => {
  const { getByText } = render(DocumentList, { props: { orgId: 'org1' } });
  await waitFor(() => {
    expect(getByText('Doc One')).toBeInTheDocument();
  });
});

test('deletes document via api', async () => {
  const { getByText, queryByText } = render(DocumentList, { props: { orgId: 'org1' } });
  await waitFor(() => {
    expect(getByText('Doc One')).toBeInTheDocument();
  });

  await getByText('Delete').click();

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/documents/1', expect.objectContaining({ method: 'DELETE' }));
    expect(queryByText('Doc One')).not.toBeInTheDocument();
  });
});
