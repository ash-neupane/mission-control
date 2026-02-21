import { useEffect, useRef, useCallback } from "react";
import { Terminal as XTerminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";
import { usePty } from "../hooks/usePty";

interface TerminalProps {
  sessionId: string;
  active: boolean; // Whether this terminal accepts input
  fontSize?: number;
  className?: string;
}

export default function Terminal({
  sessionId,
  active,
  fontSize = 13,
  className = "",
}: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<XTerminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  // Initialize xterm.js
  useEffect(() => {
    if (!containerRef.current) return;

    const term = new XTerminal({
      fontFamily:
        "JetBrains Mono, ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace",
      fontSize,
      theme: {
        background: "#0a0a0f",
        foreground: "#e0e0e8",
        cursor: active ? "#e0e0e8" : "transparent",
        cursorAccent: "#0a0a0f",
        selectionBackground: "#3b82f640",
        black: "#0a0a0f",
        red: "#ef4444",
        green: "#22c55e",
        yellow: "#f59e0b",
        blue: "#3b82f6",
        magenta: "#a855f7",
        cyan: "#06b6d4",
        white: "#e0e0e8",
        brightBlack: "#555555",
        brightRed: "#f87171",
        brightGreen: "#4ade80",
        brightYellow: "#fbbf24",
        brightBlue: "#60a5fa",
        brightMagenta: "#c084fc",
        brightCyan: "#22d3ee",
        brightWhite: "#ffffff",
      },
      cursorBlink: active,
      scrollback: 10000,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    term.open(containerRef.current);

    // Try WebGL renderer for better performance
    try {
      const webglAddon = new WebglAddon();
      term.loadAddon(webglAddon);
      webglAddon.onContextLoss(() => {
        webglAddon.dispose();
      });
    } catch {
      // Fall back to canvas renderer
    }

    fitAddon.fit();

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    return () => {
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId]); // Re-create terminal when session changes

  // Update font size and cursor
  useEffect(() => {
    if (termRef.current) {
      termRef.current.options.fontSize = fontSize;
      termRef.current.options.cursorBlink = active;
      termRef.current.options.theme = {
        ...termRef.current.options.theme,
        cursor: active ? "#e0e0e8" : "transparent",
      };
      fitAddonRef.current?.fit();
    }
  }, [fontSize, active]);

  // Handle container resize
  useEffect(() => {
    if (!containerRef.current || !fitAddonRef.current) return;

    const observer = new ResizeObserver(() => {
      try {
        fitAddonRef.current?.fit();
      } catch {
        // Ignore fit errors during transitions
      }
    });

    observer.observe(containerRef.current);

    return () => observer.disconnect();
  }, [sessionId]);

  // Connect to PTY
  const { handleResize } = usePty(sessionId, termRef.current, active);

  // Forward resize events to backend
  useEffect(() => {
    if (!termRef.current) return;

    const disposer = termRef.current.onResize(
      ({ cols, rows }: { cols: number; rows: number }) => {
        handleResize(cols, rows);
      }
    );

    return () => disposer.dispose();
  }, [sessionId, handleResize]);

  // Focus the terminal when active
  useEffect(() => {
    if (active && termRef.current) {
      termRef.current.focus();
    }
  }, [active]);

  return (
    <div
      ref={containerRef}
      className={`w-full h-full overflow-hidden ${className}`}
    />
  );
}
