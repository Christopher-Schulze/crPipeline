import { render } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import StageList from '../pipeline_editor/StageList.svelte';

const stages = [{ id: '1', type: 'ocr', ocr_engine: 'external' }];

test('shows tooltips for OCR endpoint and key', () => {
  const { getByTitle } = render(StageList, { props: { stages, availablePromptTemplates: [] } });
  expect(getByTitle('Endpoint for external OCR service')).toBeTruthy();
  expect(getByTitle('API key for the external OCR service')).toBeTruthy();
});

test('renders prompt template dropdown for AI stage', () => {
  const aiStages = [{ id: '2', type: 'ai' }];
  const templates = [{ name: 't1', text: 'x' }, { name: 't2', text: 'y' }];
  const { getByLabelText, getByText } = render(StageList, {
    props: { stages: aiStages, availablePromptTemplates: templates }
  });
  expect(getByLabelText('Prompt Template')).toBeTruthy();
  expect(getByText('t1')).toBeTruthy();
  expect(getByText('t2')).toBeTruthy();
});
