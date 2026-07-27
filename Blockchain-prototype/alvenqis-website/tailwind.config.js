/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,jsx}'],
  theme: {
    extend: {
      colors: {
        void: '#05060d',
        obsidian: '#0a0d18',
        ink: '#101427',
        ionSoft: '#38bdf8',
        ionHot: '#7dd3fc',
        plasma: '#8b5cf6',
        violetCore: '#a78bfa',
        frost: '#eef7ff',
        line: 'rgba(125, 211, 252, 0.16)',
      },
      fontFamily: {
        display: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        body: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
      },
      boxShadow: {
        ion: '0 0 90px rgba(56, 189, 248, 0.24)',
        plasma: '0 0 90px rgba(139, 92, 246, 0.20)',
        panel: '0 24px 90px rgba(0, 0, 0, 0.38)',
      },
      backgroundImage: {
        grid:
          'linear-gradient(rgba(56, 189, 248, 0.075) 1px, transparent 1px), linear-gradient(90deg, rgba(139, 92, 246, 0.055) 1px, transparent 1px)',
        radial:
          'radial-gradient(circle at 50% 0%, rgba(56, 189, 248, 0.22), rgba(139, 92, 246, 0.12) 32%, transparent 54%)',
      },
    },
  },
  plugins: [],
}
