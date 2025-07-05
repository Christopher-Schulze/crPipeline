import { render, fireEvent } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, test, vi, beforeEach } from 'vitest';
import PipelineEditor from '../PipelineEditor.svelte';
import * as apiUtils from '$lib/utils/apiUtils';

const apiFetch = vi.spyOn(apiUtils, 'apiFetch');

vi.stubGlobal('alert', vi.fn());
vi.stubGlobal('confirm', vi.fn(() => true));

const initialPipeline = { id: 'p1', name: 'Test', org_id: 'org1', stages: [{ id: 's1', type: 'parse', config: { strategy: 'Passthrough', parameters: {} } }] };

beforeEach(() => {
  apiFetch.mockResolvedValue({ ok: true, json: async () => ({ prompt_templates: [] }) });
  apiFetch.mockClear();
});

test('dispatches pipelinesUpdated and saved on save', async () => {
  const pipelinesUpdated = vi.fn();
  document.body.addEventListener('pipelinesUpdated', pipelinesUpdated);
  const { getByText, component } = render(PipelineEditor, { props: { orgId: 'org1', initialPipeline } });
  const saved = vi.fn();
  component.$on('saved', saved);

  await tick();
  await fireEvent.click(getByText('Save'));
  await tick();

  expect(pipelinesUpdated).toHaveBeenCalled();
  expect(saved).toHaveBeenCalled();

  document.body.removeEventListener('pipelinesUpdated', pipelinesUpdated);
});

test('dispatches pipelinesUpdated and saved on delete', async () => {
  const pipelinesUpdated = vi.fn();
  document.body.addEventListener('pipelinesUpdated', pipelinesUpdated);
  const { getByText, component } = render(PipelineEditor, { props: { orgId: 'org1', initialPipeline } });
  const saved = vi.fn();
  component.$on('saved', saved);

  await tick();
  await fireEvent.click(getByText('Delete'));
  await tick();

  expect(pipelinesUpdated).toHaveBeenCalled();
  expect(saved).toHaveBeenCalled();

  document.body.removeEventListener('pipelinesUpdated', pipelinesUpdated);
});
