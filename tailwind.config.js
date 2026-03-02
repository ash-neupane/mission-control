/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        cmux: {
          bg: "#0a0a0f",
          surface: "#0f0f1a",
          border: "#1a1a2e",
          "text-primary": "#e0e0e8",
          "text-secondary": "#888888",
          "text-muted": "#555555",
          working: "#3b82f6",
          "needs-input": "#f59e0b",
          "pr-ready": "#22c55e",
          done: "#1a9a4a",
          stuck: "#ef4444",
          empty: "#555555",
        },
      },
      fontFamily: {
        mono: [
          "JetBrains Mono",
          "ui-monospace",
          "SFMono-Regular",
          "SF Mono",
          "Menlo",
          "Consolas",
          "Liberation Mono",
          "monospace",
        ],
      },
      animation: {
        pulse_border: "pulse_border 2s ease-in-out infinite",
      },
      keyframes: {
        pulse_border: {
          "0%, 100%": { borderColor: "#f59e0b" },
          "50%": { borderColor: "#f59e0b80" },
        },
      },
    },
  },
  plugins: [],
};
