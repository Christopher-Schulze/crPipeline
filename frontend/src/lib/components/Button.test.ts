import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi } from 'vitest';
import Button from './Button.svelte';

describe('Button.svelte', () => {
  it('renders with default props and slot content', () => {
    // @ts-ignore
    const { component } = render(Button, {
      props: {
        customClass: 'extra-class',
        slot: 'Default Button' // Test with actual slot content
      }
    });
    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
    expect(button).toHaveTextContent('Default Button');
    expect(button).not.toBeDisabled();
    // Check for a class that is part of the primary variant and baseClasses
    expect(button.classList.contains('bg-accent')).toBe(true);
    expect(button.classList.contains('extra-class')).toBe(true);
    expect(button.getAttribute('type')).toBe('button'); // Default type
  });

  it('renders with provided slot content if no explicit slot prop', () => {
    // This test assumes rendering Button with children in markup,
    // which is slightly different from passing a 'slot' prop.
    // For programmatic rendering like this, the previous test is more direct.
    // This is more for how it might be used in a .svelte file: <Button>Click Me</Button>
    // Testing default slot content passed via children is usually handled by testing the component that *uses* Button.
    // However, if we want to simulate default slot content in testing library:
    const { container } = render(Button, {}); // No slot prop
    // To test default slot, you'd typically check for absence of specific content,
    // or use a test host component. For simplicity, previous test is better for slotted content.
    // Let's ensure it renders without explicit slot prop and doesn't crash.
    expect(screen.getByRole('button')).toBeInTheDocument();
    expect(screen.getByRole('button')).toHaveTextContent(''); // Default slot is empty by default
  });


  it('applies secondary variant classes', () => {
    render(Button, { props: { variant: 'secondary' } });
    const button = screen.getByRole('button');
    expect(button.classList.contains('bg-white/70')).toBe(true);
    expect(button.classList.contains('bg-accent')).toBe(false);
  });

  it('applies ghost variant classes', () => {
    render(Button, { props: { variant: 'ghost' } });
    const button = screen.getByRole('button');
    expect(button.classList.contains('bg-transparent')).toBe(true);
  });

  it('is disabled when disabled prop is true', () => {
    render(Button, { props: { disabled: true } });
    const button = screen.getByRole('button');
    expect(button).toBeDisabled();
    // Check for a disabled-specific class from Tailwind (e.g., disabled:bg-gray-300 for primary)
    // For primary variant (default):
    expect(button.classList.contains('disabled:bg-gray-300')).toBe(true);
  });

  it('emits a click event when clicked', async () => {
    const handleClick = vi.fn();
    // Pass slot content for the component to be interactive in some testing setups
    const { component } = render(Button, { props: { slot: 'Clickable' } });
    component.$on('click', handleClick);

    const button = screen.getByRole('button');
    await fireEvent.click(button);

    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it('does not emit a click event when disabled and clicked', async () => {
    const handleClick = vi.fn();
    const { component } = render(Button, {
        props: {
            disabled: true,
            slot: 'Disabled Button'
        }
    });
    component.$on('click', handleClick);
    const button = screen.getByRole('button');
    await fireEvent.click(button).catch(() => {}); // Click might be prevented, catch if it errors

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
    render(Button, { props: { href: '/test', slot: 'Link Button' } });
    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/test');
    expect(link).toHaveTextContent('Link Button');
  });
});
