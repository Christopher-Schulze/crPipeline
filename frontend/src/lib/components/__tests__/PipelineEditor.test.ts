import { render, fireEvent } from '@testing-library/svelte';
import { vi, expect, test } from 'vitest';

vi.mock('../../utils/apiUtils', () => ({
  apiFetch: vi.fn(() => Promise.resolve({ ok: true, json: () => Promise.resolve({ prompt_templates: [] }) }))
}));

import { apiFetch } from '../../utils/apiUtils';
import PipelineEditor from '../PipelineEditor.svelte';

const initialPipeline = {
  id: 'p1',
  name: 'Test',
  org_id: 'org1',
  stages: [{ id: 's1', type: 'parse' }]
};

test('uses apiFetch for loading templates and saving pipeline', async () => {
  const { getByText } = render(PipelineEditor, { props: { orgId: 'org1', initialPipeline } });

  expect(apiFetch).toHaveBeenCalledWith('/api/settings/org1');

  const saveBtn = getByText('Save');
  await fireEvent.click(saveBtn);

  expect(apiFetch).toHaveBeenCalledWith('/api/pipelines/p1', expect.objectContaining({ method: 'PUT' }));
});
