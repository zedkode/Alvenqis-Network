/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,jsx}'],
  theme: {
    extend: {
      colors: {
        void: '#01080d',
        obsidian: '#030c12',
        ink: '#081b26',
        ionSoft: '#00c4ef',
        ionHot: '#20d5ff',
        plasma: '#3ad4ff',
        violetCore: '#52d37e',
        gold: '#e1b05b',
        frost: '#daebf5',
        line: 'rgba(23, 61, 78, 0.78)',
      },
      fontFamily: {
        display: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        body: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
      },
      boxShadow: {
        ion: '0 0 72px rgba(32, 213, 255, 0.22)',
        plasma: '0 0 72px rgba(225, 176, 91, 0.16)',
        panel: '0 24px 90px rgba(0, 0, 0, 0.38)',
      },
      backgroundImage: {
        grid:
          'linear-gradient(rgba(32, 213, 255, 0.07) 1px, transparent 1px), linear-gradient(90deg, rgba(32, 213, 255, 0.045) 1px, transparent 1px)',
        radial:
          'radial-gradient(circle at 50% 0%, rgba(32, 213, 255, 0.2), rgba(225, 176, 91, 0.06) 34%, transparent 56%)',
      },
    },
  },
  plugins: [],
}
