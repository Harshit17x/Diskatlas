/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // Design system palette (Linear / Raycast / Professional Desktop utility)
        canvas: '#101114',
        sidebar: '#15171B',
        panel: '#1B1E23',
        elevated: '#22262C',
        border: '#2D323A',
        'border-light': '#3B414C',
        primary: '#F1F3F5',
        secondary: '#9BA3AE',
        muted: '#69717D',
        accent: {
          DEFAULT: '#5687E8',
          hover: '#4877D9',
          muted: 'rgba(86, 135, 232, 0.12)',
        },
        success: {
          DEFAULT: '#27AE82',
          muted: 'rgba(39, 174, 130, 0.12)',
        },
        warning: {
          DEFAULT: '#E5A54B',
          muted: 'rgba(229, 165, 75, 0.12)',
        },
        danger: {
          DEFAULT: '#DB6262',
          muted: 'rgba(219, 98, 98, 0.12)',
        },
      },
      fontFamily: {
        sans: [
          'Inter',
          '-apple-system',
          'BlinkMacSystemFont',
          '"Segoe UI"',
          'Roboto',
          'sans-serif',
        ],
        mono: [
          '"JetBrains Mono"',
          '"SF Mono"',
          'Menlo',
          'Consolas',
          'monospace',
        ],
      },
      borderRadius: {
        sm: '4px',
        DEFAULT: '6px',
        md: '8px',
        lg: '10px',
      },
    },
  },
  plugins: [],
}
