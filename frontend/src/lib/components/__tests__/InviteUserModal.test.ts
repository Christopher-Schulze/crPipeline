import { render } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import InviteUserModal from '../InviteUserModal.svelte';

test('renders without crashing', () => {
  const { container } = render(InviteUserModal);
  expect(container).toBeTruthy();
});
