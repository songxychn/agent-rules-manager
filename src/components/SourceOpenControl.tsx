import { useEffect, useRef, useState } from "react";
import { resolveOpenTarget } from "../lib/openTargets";
import type { OpenTarget } from "../lib/types";
import { useI18n } from "../lib/i18n";
import cursorIcon from "../assets/open-target-icons/cursor.png";
import defaultIcon from "../assets/open-target-icons/default.png";
import finderIcon from "../assets/open-target-icons/finder.png";
import intellijIdeaIcon from "../assets/open-target-icons/intellij-idea.png";
import riderIcon from "../assets/open-target-icons/rider.png";
import terminalIcon from "../assets/open-target-icons/terminal.png";
import textEditIcon from "../assets/open-target-icons/textedit.png";
import typoraIcon from "../assets/open-target-icons/typora.png";
import vscodeIcon from "../assets/open-target-icons/vscode.png";
import webstormIcon from "../assets/open-target-icons/webstorm.png";

interface SourceOpenControlProps {
  targets: OpenTarget[];
  preferredTargetId: string;
  openingTargetId?: string;
  onOpen: (targetId: string) => void;
}

const iconSources: Record<string, string> = {
  default: defaultIcon,
  vscode: vscodeIcon,
  cursor: cursorIcon,
  typora: typoraIcon,
  textedit: textEditIcon,
  "intellij-idea": intellijIdeaIcon,
  rider: riderIcon,
  webstorm: webstormIcon,
  finder: finderIcon,
  terminal: terminalIcon,
};

function OpenTargetIcon({ target }: { target: OpenTarget }) {
  return (
    <img
      className="open-target-icon"
      src={iconSources[target.id] ?? defaultIcon}
      alt=""
      aria-hidden="true"
      draggable={false}
    />
  );
}

export function SourceOpenControl({
  targets,
  preferredTargetId,
  openingTargetId,
  onOpen,
}: SourceOpenControlProps) {
  const { t } = useI18n();
  const [menuOpen, setMenuOpen] = useState(false);
  const pickerRef = useRef<HTMLDivElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const firstItemRef = useRef<HTMLButtonElement>(null);
  const focusFirstOnOpen = useRef(false);
  const activeTarget = resolveOpenTarget(targets, preferredTargetId);

  const targetLabel = (target: OpenTarget) =>
    target.id === "default" ? t("source.open.default") : target.label;
  const primaryLabel = openingTargetId
    ? t("source.open.opening")
    : activeTarget
      ? t("source.open.with", { app: targetLabel(activeTarget) })
      : t("source.open.unavailable");

  useEffect(() => {
    if (!menuOpen) return;
    if (focusFirstOnOpen.current) firstItemRef.current?.focus();
    focusFirstOnOpen.current = false;

    const closeOnPointerDown = (event: PointerEvent) => {
      if (!pickerRef.current?.contains(event.target as Node)) setMenuOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenuOpen(false);
        toggleRef.current?.focus();
      }
    };
    window.addEventListener("pointerdown", closeOnPointerDown);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", closeOnPointerDown);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [menuOpen]);

  const chooseTarget = (target: OpenTarget) => {
    setMenuOpen(false);
    onOpen(target.id);
  };

  const renderTarget = (target: OpenTarget, index: number) => {
    const selected = target.id === activeTarget?.id;
    return (
      <button
        className={`open-target-option ${selected ? "is-selected" : ""}`}
        key={target.id}
        ref={index === 0 ? firstItemRef : undefined}
        role="menuitemradio"
        aria-checked={selected}
        onClick={() => chooseTarget(target)}
      >
        <OpenTargetIcon target={target} />
        <span>{targetLabel(target)}</span>
      </button>
    );
  };

  return (
    <div className="open-target-picker" ref={pickerRef}>
      <div className="open-target-split">
        <button
          className="open-target-primary"
          disabled={!activeTarget || Boolean(openingTargetId)}
          aria-label={primaryLabel}
          title={primaryLabel}
          onClick={() => activeTarget && onOpen(activeTarget.id)}
        >
          {activeTarget ? (
            <OpenTargetIcon target={activeTarget} />
          ) : (
            <img
              className="open-target-icon"
              src={defaultIcon}
              alt=""
              aria-hidden="true"
              draggable={false}
            />
          )}
        </button>
        <button
          className="open-target-toggle"
          ref={toggleRef}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          aria-label={t("source.open.menu")}
          disabled={!targets.length || Boolean(openingTargetId)}
          onClick={(event) => {
            focusFirstOnOpen.current = event.detail === 0;
            setMenuOpen((current) => !current);
          }}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="m4 6 4 4 4-4" />
          </svg>
        </button>
      </div>

      {menuOpen && (
        <div className="open-target-menu" role="menu" aria-label={t("source.open.menu")}>
          {targets.map(renderTarget)}
        </div>
      )}
    </div>
  );
}
