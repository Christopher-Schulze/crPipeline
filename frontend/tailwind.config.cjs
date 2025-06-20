module.exports = {
  content: ['./src/**/*.{svelte,ts}'],
  theme: {
    extend: {
      colors: {
        accent: 'var(--color-accent, #30D5C8)',
        base: '#f2f2f5',
        error: '#FF3B30',
        success: '#34C759'
      }
    }
  },
  plugins: [],
};
