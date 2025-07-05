import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

// jsdom's window.confirm throws a not implemented error. Override with a stub.
if (typeof window !== 'undefined') {
  Object.defineProperty(window, 'confirm', {
    writable: true,
    configurable: true,
    value: vi.fn(() => true)
  });
}
