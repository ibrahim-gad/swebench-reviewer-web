/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.rs",
    "./style/**/*.{css,scss,sass}",
  ],
  darkMode: 'class',
  theme: {
    extend: {},
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
