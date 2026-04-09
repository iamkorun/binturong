module.exports = {
  content: ['./index.html'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        char: { 900: '#14120E', 800: '#1C1A17', 700: '#25221D', 600: '#2F2B25', 500: '#3C372F' },
        cream: { 50: '#FFFBF5', 100: '#FAF6EF', 200: '#F1EBDF', 300: '#E5DDCD' },
        amber: { DEFAULT: '#E4A43A', hi: '#F2B347', lo: '#B87E1C' },
        sage: '#7A8F5E',
        rust: '#C3553A',
        stone: { 400: '#A39A8A', 500: '#7D7568', 600: '#5C564B' },
      },
      fontFamily: {
        sans: ['Geist', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        mono: ['"Geist Mono"', 'ui-monospace', 'JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
};
