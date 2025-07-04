import { render, fireEvent } from '@testing-library/svelte';
import { vi, expect, test } from 'vitest';
import { tick } from 'svelte';
import UploadForm from '../UploadForm.svelte';

vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: true })) as any);

test('emits uploaded event after successful fetch', async () => {
  const { container, component } = render(UploadForm, { props: { orgId: '1', userId: 'u1', pipelineId: 'p1' } });
  const handler = vi.fn();
  component.$on('uploaded', handler);
  const input = container.querySelector('input[type="file"]') as HTMLInputElement;
  Object.defineProperty(input, 'files', {
    value: [new File(['hi'], 'test.txt', { type: 'text/plain' })],
  });
  await fireEvent.change(input);
  await tick();
  expect(fetch).toHaveBeenCalled();
  expect(handler).toHaveBeenCalled();
});
