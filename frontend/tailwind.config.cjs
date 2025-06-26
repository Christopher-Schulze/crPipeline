module.exports = {
  content: ['./src/**/*.{svelte,ts}'],
  theme: {
    extend: {
      colors: {
        accent: 'var(--color-accent, #30D5C8)',
        base: '#f2f2f5', // Light theme base
        error: '#FF3B30',
        success: '#34C759'
      },
      fontFamily: {
        sans: [
          'SF Pro Display',
          // Fallback to system fonts
          '-apple-system',
          'BlinkMacSystemFont',
          '"Segoe UI"',
          'Roboto',
          '"Helvetica Neue"',
          'Arial',
          '"Noto Sans"',
          'sans-serif',
          // Emoji fallbacks
          '"Apple Color Emoji"',
          '"Segoe UI Emoji"',
          '"Segoe UI Symbol"',
          '"Noto Color Emoji"',
        ],
      },
    }
  },
  plugins: [],
};
