/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.{html,js,svelte,ts}',
    './src/routes/**/*.{svelte,js,ts}',
    './src/components/**/*.{svelte,js,ts}',
    './src/app.html'
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['"IBM Plex Sans"', 'system-ui', 'sans-serif'],
        mono: ['"Google Sans Mono"', 'ui-monospace', 'monospace'],
      },
    },
  },
  plugins: [],
}