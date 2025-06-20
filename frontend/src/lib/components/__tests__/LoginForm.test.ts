import { render, fireEvent } from '@testing-library/svelte';
import LoginForm from '../LoginForm.svelte';
import { vi, expect, test } from 'vitest';
import { tick } from 'svelte';

vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: true })) as any);

test('emits loggedin on successful submit', async () => {
  const { getByText, component } = render(LoginForm);
  const handler = vi.fn();
  component.$on('loggedin', handler);
  await fireEvent.click(getByText('Login'));
  await tick();
  expect(handler).toHaveBeenCalled();
});
