import type { Config } from 'tailwindcss'

export default {
  darkMode: ['class'],
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    container: {
      center: true,
      padding: '1rem',
      screens: {
        '2xl': '1400px',
      },
    },
    extend: {
      colors: {
        border: 'hsl(222 20% 20%)',
        input: 'hsl(222 20% 18%)',
        ring: 'hsl(207 92% 67%)',
        background: 'hsl(224 24% 8%)',
        foreground: 'hsl(210 25% 95%)',
        primary: {
          DEFAULT: 'hsl(207 92% 67%)',
          foreground: 'hsl(224 24% 8%)',
        },
        secondary: {
          DEFAULT: 'hsl(223 17% 17%)',
          foreground: 'hsl(210 25% 95%)',
        },
        muted: {
          DEFAULT: 'hsl(223 17% 14%)',
          foreground: 'hsl(216 12% 70%)',
        },
        card: {
          DEFAULT: 'hsl(222 22% 12%)',
          foreground: 'hsl(210 25% 95%)',
        },
        success: 'hsl(152 57% 57%)',
        warning: 'hsl(39 89% 63%)',
        danger: 'hsl(357 86% 67%)',
      },
      borderRadius: {
        lg: '0.9rem',
        md: '0.65rem',
        sm: '0.45rem',
      },
      boxShadow: {
        glow: '0 0 0 1px hsl(207 92% 67% / 0.18), 0 18px 48px hsl(224 42% 4% / 0.45)',
      },
    },
  },
  plugins: [],
} satisfies Config
