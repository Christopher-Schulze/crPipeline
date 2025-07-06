# Theming

## Accent color
The global CSS variable `--color-accent` drives the main highlight color of the UI.
Each organization has an `accent_color` stored in its settings table. When the
frontend loads `/api/settings/{org_id}` the color is applied with

```ts
document.documentElement.style.setProperty('--color-accent', data.accent_color);
```

The Settings form emits a `saved` event that updates the variable the same way
once the user changes the color.

## DaisyUI themes
The accent color is referenced in `frontend/tailwind.config.cjs` and fed into
DaisyUI. The configuration extends the color palette and defines the themes:

```js
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
  plugins: [require('daisyui')],
  daisyui: {
    themes: [
      {
        light: {
          primary: 'var(--color-accent, #30D5C8)',
          accent: 'var(--color-accent, #30D5C8)',
          'base-100': '#f2f2f5',
          error: '#FF3B30',
          success: '#34C759'
        }
      },
      'dark'
    ]
  }
};
```

Change any of these values or add new objects inside `themes` to customize the
look. A simple variation might set a different default accent color:

```js
  daisyui: {
    themes: [
      {
        light: {
          primary: 'var(--color-accent, #9747FF)',
          accent: 'var(--color-accent, #9747FF)',
          'base-100': '#f3f4f6'
        }
      },
      'dark'
    ]
  }
```

Rebuild the frontend with `npm run build:prod --prefix frontend` so DaisyUI
compiles the updated themes.

## Organization settings
Organization admins can adjust the accent color under **Settings**. The API
endpoints `/api/settings/{org_id}` (GET) and `/api/settings` (POST) persist the
value. Each organization therefore gets its own highlight color once users log
in and the frontend applies `--color-accent` accordingly.
