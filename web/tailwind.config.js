/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        // A conversational instrument reads better in a serif; the record
        // and the token are monospaced because they are meant to be copied.
        sans: ['Inter', 'system-ui', 'sans-serif'],
        serif: ['Charter', 'Georgia', 'serif'],
        mono: ['ui-monospace', 'SFMono-Regular', 'Menlo', 'monospace'],
      },
    },
  },
  plugins: [],
}
