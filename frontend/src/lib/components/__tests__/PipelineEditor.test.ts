import { render, fireEvent } from '@testing-library/svelte';
import { vi, expect, test } from 'vitest';
import { tick } from 'svelte';

import * as apiModule from '../../utils/apiUtils';
const apiFetch = vi.spyOn(apiModule, 'apiFetch').mockResolvedValue({
  ok: true,
  json: () => Promise.resolve({ prompt_templates: [] })
});
import PipelineEditor from '../PipelineEditor.svelte';

const initialPipeline = {
  id: 'p1',
  name: 'Test',
  org_id: 'org1',
  stages: [{ id: 's1', type: 'parse', config: { strategy: 'Passthrough', parameters: {} } }]
};

test('uses apiFetch for loading templates, saving and deleting pipeline', async () => {
  const { getByText } = render(PipelineEditor, { props: { orgId: 'org1', initialPipeline } });
  await tick();
  await new Promise(r => setTimeout(r, 0));

  vi.spyOn(window, 'alert').mockImplementation(() => {});

  const saveBtn = getByText('Save');
  await fireEvent.click(saveBtn);

  expect(apiFetch).toHaveBeenCalledWith('/api/pipelines/p1', expect.objectContaining({ method: 'PUT' }));

  vi.spyOn(window, 'confirm').mockReturnValue(true);

  const deleteBtn = getByText('Delete');
  await fireEvent.click(deleteBtn);

  expect(apiFetch).toHaveBeenCalledWith('/api/pipelines/p1', expect.objectContaining({ method: 'DELETE' }));
});
