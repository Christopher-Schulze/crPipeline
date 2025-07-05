import { render, waitFor } from '@testing-library/svelte';
import { vi, expect, test } from 'vitest';
import Dashboard from '../Dashboard.svelte';
import { errorStore } from '$lib/utils/errorStore';

vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: true, json: async () => ({ upload_remaining: 0, analysis_remaining: 0 }) })) as any);

test('renders dashboard cards', () => {
  const { getByText } = render(Dashboard, { props: { orgId: '' } });
  expect(getByText('Uploads Remaining')).toBeInTheDocument();
  expect(getByText('Analyses Remaining')).toBeInTheDocument();
});

test('shows error message when fetch fails', async () => {
  const showSpy = vi.spyOn(errorStore, 'show');
  (fetch as any).mockResolvedValue({ ok: false, status: 500, json: async () => ({ error: 'oops' }) });
  render(Dashboard, { props: { orgId: '1' } });
  await waitFor(() => {
    expect(showSpy).toHaveBeenCalledWith('Failed to load quota: oops');
  });
});
