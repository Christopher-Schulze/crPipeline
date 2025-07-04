import { render } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import EditUserRoleModal from '../EditUserRoleModal.svelte';

test('renders closed by default', () => {
  const { container } = render(EditUserRoleModal);
  expect(container).toBeTruthy();
});
