import { render, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';

import { sessionStore } from '../../stores/session';

beforeEach(() => {
  sessionStore.setSession({ loggedIn: true, org: 'org1', userId: null, role: null, csrfToken: null });
});

afterEach(() => {
  sessionStore.clear();
});
import PipelinesPage from '../../../routes/pipelines/+page.svelte';

const pipelines = [{ id: 'p1', name: 'Pipe', org_id: 'org1', stages: [] }];

const fetchMock = vi.fn((url: string, options?: any) => {
  if (url === '/api/pipelines/org1') {
    return Promise.resolve({ ok: true, json: async () => pipelines.slice() });
  }
  if (url === '/api/pipelines/p1' && options?.method === 'DELETE') {
    pipelines.pop();
    return Promise.resolve({ ok: true, text: async () => '' });
  }
  return Promise.resolve({ ok: true, json: async () => ({}) });
}) as any;

vi.stubGlobal('fetch', fetchMock);

test('deletes pipeline via api', async () => {
  const { getByText, queryByText } = render(PipelinesPage);

  await waitFor(() => expect(getByText('Pipe')).toBeInTheDocument());

  await getByText('Delete').click();

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/pipelines/p1', expect.objectContaining({ method: 'DELETE' }));
    expect(queryByText('Pipe')).not.toBeInTheDocument();
  });
});
