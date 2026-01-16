/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.{html,js,svelte,ts}',
    './src/routes/**/*.{svelte,js,ts}',
    './src/components/**/*.{svelte,js,ts}',
    './src/app.html'
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}