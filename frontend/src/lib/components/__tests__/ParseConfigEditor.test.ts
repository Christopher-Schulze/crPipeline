import { render } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import ParseConfigEditor from '../pipeline_editor/ParseConfigEditor.svelte';

const stage = { id: '1', type: 'parse', config: { strategy: 'Passthrough', parameters: {} } };
const init = () => {};

test('renders parse config editor', () => {
  const { container } = render(ParseConfigEditor, { props: { stage, initializeParseStrategyParameters: init } });
  expect(container).toBeTruthy();
});
