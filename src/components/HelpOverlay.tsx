import { useStore } from "../store";

export default function HelpOverlay() {
  const { showHelpOverlay, toggleHelpOverlay } = useStore();

  if (!showHelpOverlay) return null;

  return (
    <div
      className="fixed inset-0 bg-black/70 flex items-center justify-center z-50"
      onClick={toggleHelpOverlay}
    >
      <div
        className="bg-cmux-surface border border-cmux-border rounded-lg w-[500px] p-6 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="text-[14px] font-bold text-cmux-text-primary mb-4">
          c-mux Keyboard Shortcuts
        </h2>

        <div className="grid grid-cols-2 gap-6">
          <section>
            <h3 className="text-[10px] font-bold uppercase text-cmux-text-muted mb-2 tracking-wider">
              Overview Mode
            </h3>
            <div className="space-y-1.5">
              <HelpRow keys="1-9" desc="Focus session N" />
              <HelpRow keys="↑↓←→" desc="Select session" />
              <HelpRow keys="Enter" desc="Focus selected session" />
              <HelpRow keys="Tab" desc="Next needs-input session" />
              <HelpRow keys="n" desc="New session" />
              <HelpRow keys="q" desc="Kill selected session" />
              <HelpRow keys="?" desc="Toggle help" />
            </div>
          </section>

          <section>
            <h3 className="text-[10px] font-bold uppercase text-cmux-text-muted mb-2 tracking-wider">
              Focus Mode
            </h3>
            <div className="space-y-1.5">
              <HelpRow keys="Esc Esc" desc="Return to overview" />
              <HelpRow keys="Alt+1-9" desc="Switch to session N" />
              <HelpRow keys="Alt+Tab" desc="Next needs-input session" />
              <HelpRow keys="Ctrl+Q" desc="Kill session" />
              <HelpRow keys="Ctrl+P" desc="Open PR in browser" />
              <HelpRow keys="Ctrl+B" desc="Toggle side panel" />
              <HelpRow keys="Ctrl+N" desc="New session" />
            </div>
          </section>

          <section>
            <h3 className="text-[10px] font-bold uppercase text-cmux-text-muted mb-2 tracking-wider">
              New Session
            </h3>
            <div className="space-y-1.5">
              <HelpRow keys="↑↓" desc="Navigate projects" />
              <HelpRow keys="Enter" desc="Launch session" />
              <HelpRow keys="/path" desc="Add & launch project" />
              <HelpRow keys="Tab" desc="Edit branch" />
              <HelpRow keys="Esc" desc="Cancel" />
            </div>
          </section>

          <section>
            <h3 className="text-[10px] font-bold uppercase text-cmux-text-muted mb-2 tracking-wider">
              Design Principle
            </h3>
            <p className="text-[10px] text-cmux-text-secondary leading-relaxed">
              In focus mode, all shortcuts use modifier keys
              (Ctrl, Alt) so bare keystrokes pass through to
              the terminal unintercepted.
            </p>
          </section>
        </div>

        <div className="mt-4 pt-3 border-t border-cmux-border text-center">
          <span className="text-[10px] text-cmux-text-muted">
            Press{" "}
            <kbd className="px-1 py-0.5 rounded bg-cmux-border text-cmux-text-secondary text-[9px]">
              ?
            </kbd>{" "}
            or{" "}
            <kbd className="px-1 py-0.5 rounded bg-cmux-border text-cmux-text-secondary text-[9px]">
              Esc
            </kbd>{" "}
            to close
          </span>
        </div>
      </div>
    </div>
  );
}

function HelpRow({ keys, desc }: { keys: string; desc: string }) {
  return (
    <div className="flex items-center gap-2 text-[11px]">
      <kbd className="px-1.5 py-0.5 rounded bg-cmux-border text-cmux-text-secondary text-[10px] font-mono min-w-[60px] text-center">
        {keys}
      </kbd>
      <span className="text-cmux-text-secondary">{desc}</span>
    </div>
  );
}
