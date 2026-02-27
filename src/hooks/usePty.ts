import { useEffect, useRef, useCallback } from "react";
import type { Terminal } from "@xterm/xterm";
import { onPtyOutput, writeToPty, resizePty } from "../lib/tauri";

/**
 * Hook that connects an xterm.js Terminal instance to a Tauri PTY session.
 * Handles subscribing to PTY output events and forwarding terminal input.
 */
export function usePty(
  sessionId: string | null,
  terminal: Terminal | null,
  active: boolean
) {
  const unlistenRef = useRef<(() => void) | null>(null);
  const dataDisposerRef = useRef<{ dispose: () => void } | null>(null);

  // Subscribe to PTY output
  useEffect(() => {
    if (!sessionId || !terminal) return;

    let cancelled = false;

    const setup = async () => {
      // Listen for PTY output from the Rust backend
      const unlisten = await onPtyOutput(sessionId, (data: number[]) => {
        if (cancelled) return;
        const bytes = new Uint8Array(data);
        terminal.write(bytes);
      });

      if (cancelled) {
        unlisten();
        return;
      }

      unlistenRef.current = unlisten;
    };

    setup().catch((err) =>
      console.error(`Failed to subscribe to PTY output for ${sessionId}:`, err)
    );

    return () => {
      cancelled = true;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [sessionId, terminal]);

  // Forward terminal input to PTY
  useEffect(() => {
    if (!sessionId || !terminal || !active) return;

    const disposer = terminal.onData((data: string) => {
      const encoder = new TextEncoder();
      const bytes = Array.from(encoder.encode(data));
      writeToPty(sessionId, bytes).catch(console.error);
    });

    dataDisposerRef.current = disposer;

    return () => {
      disposer.dispose();
      dataDisposerRef.current = null;
    };
  }, [sessionId, terminal, active]);

  // Handle terminal resize
  const handleResize = useCallback(
    (cols: number, rows: number) => {
      if (!sessionId) return;
      resizePty(sessionId, cols, rows).catch(console.error);
    },
    [sessionId]
  );

  return { handleResize };
}
