import { render } from '@testing-library/svelte';
import { vi, expect, test } from 'vitest';
import Dashboard from '../Dashboard.svelte';

vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: true, json: async () => ({ upload_remaining: 0, analysis_remaining: 0 }) })) as any);

test('renders dashboard cards', () => {
  const { getByText } = render(Dashboard, { props: { orgId: '' } });
  expect(getByText('Uploads Remaining')).toBeInTheDocument();
  expect(getByText('Analyses Remaining')).toBeInTheDocument();
});
