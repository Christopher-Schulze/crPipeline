import { render, waitFor, fireEvent } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import DocumentList from '../DocumentList.svelte';

const fetchMock = vi.fn((url: string) => {
  if (url.startsWith('/api/download/')) {
    return Promise.resolve({
      ok: true,
      json: async () => ({ url: 'http://example.com/file.pdf' })
    });
  }
  return Promise.resolve({
    ok: true,
    json: async () => ({
      items: [
        {
          id: '1',
          filename: 'doc1.pdf',
          display_name: 'Doc One',
          is_target: true,
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
  });
}) as any;

vi.stubGlobal('fetch', fetchMock);


// window.open is not global open? It's window.open
const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);

// Test download button triggers API call and opens link

test('download button uses API and opens link', async () => {
  const { getByText } = render(DocumentList, { props: { orgId: 'org1' } });

  await waitFor(() => expect(getByText('Doc One')).toBeInTheDocument());

  await fireEvent.click(getByText('Download'));

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/download/1', { credentials: 'include' });
    expect(openSpy).toHaveBeenCalledWith('http://example.com/file.pdf', '_blank');
  });
});

