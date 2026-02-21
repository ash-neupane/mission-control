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

export function statusBorderClass(status: SessionStatus): string {
  switch (status) {
    case "Working":
      return "border-l-4 border-l-cmux-working";
    case "NeedsInput":
      return "border-l-4 border-l-cmux-needs-input animate-pulse_border";
    case "PrReady":
      return "border-l-4 border-l-cmux-pr-ready";
    case "Done":
      return "border-l-4 border-l-cmux-done";
    case "Stuck":
      return "border-l-4 border-l-cmux-stuck";
    default:
      return "border-l-4 border-l-cmux-empty";
  }
}
