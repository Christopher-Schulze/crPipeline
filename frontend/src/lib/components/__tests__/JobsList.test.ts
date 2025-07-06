import { render, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import JobsList from '../JobsList.svelte';

const now = new Date().toISOString();

test('shows jobs fetched from api', async () => {
  vi.stubGlobal('EventSource', class { close() {} } as any);
  const fetchMock = vi.fn(() => Promise.resolve({
    ok: true,
    json: async () => ({
      items: [{ id: 'j1', org_id: 'o1', document_id: 'd1', pipeline_id: 'p1', status: 'pending', created_at: now, document_name: 'Doc.pdf', pipeline_name: 'MyPipe' }],
      page: 1,
      per_page: 10,
      total_items: 1,
      total_pages: 1,
    })
  })) as any;
  vi.stubGlobal('fetch', fetchMock);
  const { getByText } = render(JobsList, { props: { orgId: 'o1' } });
  await waitFor(() => {
    expect(getByText('Doc.pdf')).toBeInTheDocument();
  });
  expect(getByText('MyPipe')).toBeInTheDocument();
});
