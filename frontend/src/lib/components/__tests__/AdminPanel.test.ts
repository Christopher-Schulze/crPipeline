import { render, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import * as apiUtils from '$lib/utils/apiUtils';
import OrgAdmin from '../OrgAdmin.svelte';

const apiFetch = vi.spyOn(apiUtils, 'apiFetch').mockResolvedValue({
  ok: true,
  json: async () => []
});

test('renders admin panel tabs', async () => {
  const { getByText } = render(OrgAdmin, { props: { currentUserId: '1' } });
  await waitFor(() => {
    expect(getByText('Organizations')).toBeInTheDocument();
    expect(getByText('User Management')).toBeInTheDocument();
  });
});
