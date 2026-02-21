import type { SessionStatus } from "../types";

export const statusColors: Record<SessionStatus, string> = {
  Empty: "#555555",
  Working: "#3b82f6",
  NeedsInput: "#f59e0b",
  PrReady: "#22c55e",
  Done: "#1a9a4a",
  Stuck: "#ef4444",
};

export const statusLabels: Record<SessionStatus, string> = {
  Empty: "EMPTY",
  Working: "WORK",
  NeedsInput: "ASK",
  PrReady: "PR",
  Done: "DONE",
  Stuck: "STUCK",
};

const statusBorderClasses: Record<SessionStatus, string> = {
  Empty: "border-l-4 border-l-cmux-empty",
  Working: "border-l-4 border-l-cmux-working",
  NeedsInput: "border-l-4 border-l-cmux-needs-input animate-pulse_border",
  PrReady: "border-l-4 border-l-cmux-pr-ready",
  Done: "border-l-4 border-l-cmux-done",
  Stuck: "border-l-4 border-l-cmux-stuck",
};

export function statusBorderClass(status: SessionStatus): string {
  return statusBorderClasses[status];
}
