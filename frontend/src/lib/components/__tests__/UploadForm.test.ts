import { render, fireEvent, waitFor } from '@testing-library/svelte';
import { vi, expect, test } from 'vitest';
import { tick } from 'svelte';
import UploadForm from '../UploadForm.svelte';
import { errorStore } from '$lib/utils/errorStore';

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
  await waitFor(() => {
    expect(handler).toHaveBeenCalled();
  });
});

test('displays error message on failed upload', async () => {
  const showSpy = vi.spyOn(errorStore, 'show');
  (fetch as any).mockResolvedValueOnce({ ok: false, status: 400, json: async () => ({ error: 'Bad' }) });
  const { container } = render(UploadForm, { props: { orgId: '1', userId: 'u1', pipelineId: null } });
  const input = container.querySelector('input[type="file"]') as HTMLInputElement;
  Object.defineProperty(input, 'files', {
    value: [new File(['hi'], 'test.txt', { type: 'text/plain' })],
  });
  await fireEvent.change(input);
  await tick();
  expect(showSpy).toHaveBeenCalled();
});
