import { useStore } from "../store";

export default function StatusBar() {
  const { viewMode } = useStore();

  if (viewMode === "focus") {
    return (
      <div className="flex items-center h-6 px-3 bg-cmux-surface border-t border-cmux-border text-[10px] text-cmux-text-muted select-none">
        <div className="flex items-center gap-4">
          <Hotkey keys="esc esc" label="overview" />
          <Hotkey keys="alt+1-9" label="switch" />
          <Hotkey keys="alt+tab" label="next ask" />
          <Hotkey keys="ctrl+q" label="kill" />
          <Hotkey keys="ctrl+p" label="open PR" />
          <Hotkey keys="ctrl+b" label="panel" />
          <Hotkey keys="ctrl+/" label="help" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center h-6 px-3 bg-cmux-surface border-t border-cmux-border text-[10px] text-cmux-text-muted select-none">
      <div className="flex items-center gap-4">
        <Hotkey keys="1-9" label="focus" />
        <Hotkey keys="arrows" label="select" />
        <Hotkey keys="enter" label="open" />
        <Hotkey keys="n" label="new" />
        <Hotkey keys="q" label="kill" />
        <Hotkey keys="tab" label="next ask" />
        <Hotkey keys="?" label="help" />
      </div>
    </div>
  );
}

function Hotkey({ keys, label }: { keys: string; label: string }) {
  return (
    <span className="flex items-center gap-1">
      <kbd className="px-1 py-0.5 rounded bg-cmux-border text-cmux-text-secondary text-[9px] font-bold">
        {keys}
      </kbd>
      <span>{label}</span>
    </span>
  );
}
