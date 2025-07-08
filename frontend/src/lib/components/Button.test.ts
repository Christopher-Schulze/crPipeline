import { render, screen, fireEvent } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi } from 'vitest';
import Button from './Button.svelte';

describe('Button.svelte', () => {
  it('renders with default props', () => {
    render(Button, { props: { customClass: 'extra-class' } });
    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
    expect(button).not.toBeDisabled();
    // Should use the primary variant styling by default
    expect(button.classList.contains('btn-primary')).toBe(true);
    expect(button.classList.contains('extra-class')).toBe(true);
    expect(button.getAttribute('type')).toBe('button'); // Default type
  });

  it('renders without slot content', () => {
    render(Button, {});
    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
    expect(button).toHaveTextContent('');
  });


  it('applies secondary variant classes', () => {
    render(Button, { props: { variant: 'secondary' } });
    const button = screen.getByRole('button');
    expect(button.classList.contains('btn-secondary')).toBe(true);
    expect(button.classList.contains('btn-primary')).toBe(false);
  });

  it('applies ghost variant classes', () => {
    render(Button, { props: { variant: 'ghost' } });
    const button = screen.getByRole('button');
    expect(button.classList.contains('btn-ghost')).toBe(true);
  });

  it('applies danger variant classes', () => {
    render(Button, { props: { variant: 'danger' } });
    const button = screen.getByRole('button');
    expect(button.classList.contains('btn-error')).toBe(true);
  });

  it('is disabled when disabled prop is true', () => {
    render(Button, { props: { disabled: true } });
    const button = screen.getByRole('button');
    expect(button).toBeDisabled();
    // Disabled buttons keep the primary styling
    expect(button.classList.contains('btn-primary')).toBe(true);
  });

  it('emits a click event when clicked', async () => {
    const handleClick = vi.fn();
    // Pass slot content for the component to be interactive in some testing setups
    const { component } = render(Button, {});
    component.$on('click', handleClick);

    const button = screen.getByRole('button');
    await fireEvent.click(button);

    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it('does not emit a click event when disabled and clicked', async () => {
    const handleClick = vi.fn();
    const { component } = render(Button, { props: { disabled: true } });
    component.$on('click', handleClick);
    const button = screen.getByRole('button');
    await userEvent.click(button);

    expect(handleClick).not.toHaveBeenCalled();
  });

  it('renders with a different type', () => {
    render(Button, { props: { type: 'submit' } });
    const button = screen.getByRole('button');
    expect(button).toHaveAttribute('type', 'submit');
  });

  it('renders custom classes when provided', () => {
    render(Button, { props: { customClass: 'my-custom-class another-class' } });
    const button = screen.getByRole('button');
    expect(button.classList.contains('my-custom-class')).toBe(true);
    expect(button.classList.contains('another-class')).toBe(true);
  });

  it('renders an anchor when href is provided', () => {
    render(Button, { props: { href: '/test' } });
    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/test');
    expect(link).toHaveTextContent('');
  });
});
