import { useEffect, useMemo, useRef, useState } from "react";
import { groupOpenTargets, resolveOpenTarget } from "../lib/openTargets";
import type { OpenTarget } from "../lib/types";
import { useI18n } from "../lib/i18n";

interface SourceFilePanelProps {
  sourcePath: string;
  sourceDigest?: string;
  sourceModifiedAt?: string;
  targets: OpenTarget[];
  preferredTargetId: string;
  openingTargetId?: string;
  refreshing: boolean;
  onOpen: (targetId: string) => void;
  onRefresh: () => void;
}

const iconMarks: Record<string, string> = {
  default: "↗",
  vscode: "VS",
  cursor: "◆",
  typora: "T",
  textedit: "A",
  notepad: "N",
  "intellij-idea": "IJ",
  rider: "RD",
  webstorm: "WS",
  finder: "FI",
  explorer: "EX",
  files: "FL",
  terminal: ">_",
  "windows-terminal": ">_",
  konsole: ">_",
  "xfce-terminal": ">_",
};

function OpenTargetIcon({ target }: { target: OpenTarget }) {
  return (
    <span className={`open-target-icon open-target-${target.id}`} aria-hidden="true">
      {iconMarks[target.id] ?? target.label.slice(0, 2).toUpperCase()}
    </span>
  );
}

export function SourceFilePanel({
  sourcePath,
  sourceDigest,
  sourceModifiedAt,
  targets,
  preferredTargetId,
  openingTargetId,
  refreshing,
  onOpen,
  onRefresh,
}: SourceFilePanelProps) {
  const { locale, t } = useI18n();
  const [menuOpen, setMenuOpen] = useState(false);
  const pickerRef = useRef<HTMLDivElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const firstItemRef = useRef<HTMLButtonElement>(null);
  const focusFirstOnOpen = useRef(false);
  const activeTarget = resolveOpenTarget(targets, preferredTargetId);
  const groups = useMemo(() => groupOpenTargets(targets), [targets]);
  const fileName = sourcePath.split(/[\\/]/).pop() || "AGENTS.md";
  const modifiedAt = sourceModifiedAt
    ? new Intl.DateTimeFormat(locale === "zh-CN" ? "zh-CN" : "en-US", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(new Date(sourceModifiedAt))
    : t("source.file.unknownTime");

  const targetLabel = (target: OpenTarget) =>
    target.id === "default" ? t("source.open.default") : target.label;

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
        <span className="open-target-check" aria-hidden="true">
          {selected ? "✓" : ""}
        </span>
      </button>
    );
  };

  return (
    <section className="source-file-panel" aria-labelledby="source-file-heading">
      <div className="source-file-toolbar">
        <div>
          <p className="eyebrow">{t("source.file.eyebrow")}</p>
          <h2 id="source-file-heading">{t("source.file.title")}</h2>
        </div>

        <div className="open-target-picker" ref={pickerRef}>
          <div className="open-target-split">
            <button
              className="open-target-primary"
              disabled={!activeTarget || Boolean(openingTargetId)}
              onClick={() => activeTarget && onOpen(activeTarget.id)}
              title={
                activeTarget
                  ? t("source.open.with", { app: targetLabel(activeTarget) })
                  : t("source.open.unavailable")
              }
            >
              {activeTarget && <OpenTargetIcon target={activeTarget} />}
              <span className="open-target-primary-copy">
                <small>{openingTargetId ? t("source.open.opening") : t("source.open.action")}</small>
                <strong>
                  {activeTarget ? targetLabel(activeTarget) : t("source.open.unavailable")}
                </strong>
              </span>
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
              <span aria-hidden="true">⌄</span>
            </button>
          </div>

          {menuOpen && (
            <div className="open-target-menu" role="menu" aria-label={t("source.open.menu")}>
              <p className="open-target-group-label">{t("source.open.editors")}</p>
              {groups.editors.map(renderTarget)}
              {groups.system.length > 0 && (
                <>
                  <div className="open-target-divider" role="separator" />
                  <p className="open-target-group-label">{t("source.open.system")}</p>
                  {groups.system.map((target, index) => renderTarget(target, index + 100))}
                </>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="path-strip" title={sourcePath}>
        <span>{t("source.file.path")}</span>
        <code>{sourcePath}</code>
      </div>

      <div className="source-file-body">
        <div className="source-document-row">
          <span className="source-document-mark" aria-hidden="true">
            <span>MD</span>
          </span>
          <div>
            <strong>{fileName}</strong>
            <p>{t("source.file.description")}</p>
          </div>
        </div>

        <dl className="source-file-metadata">
          <div>
            <dt>{t("source.file.digest")}</dt>
            <dd>
              <code>{sourceDigest ?? t("projection.uninitialized")}</code>
            </dd>
          </div>
          <div>
            <dt>{t("source.file.modified")}</dt>
            <dd>{modifiedAt}</dd>
          </div>
        </dl>

        <div className="source-boundary-note">
          <span className="source-boundary-glyph" aria-hidden="true">
            ↗
          </span>
          <div>
            <strong>{t("source.file.externalTitle")}</strong>
            <p>{t("source.file.externalBody")}</p>
          </div>
        </div>
      </div>

      <footer className="source-file-footer">
        <span className="source-watch-state">
          <span aria-hidden="true" />
          {t("source.file.watch")}
        </span>
        <button
          className="button button-secondary button-small"
          disabled={refreshing}
          onClick={onRefresh}
        >
          {refreshing ? t("source.file.refreshing") : t("source.file.refresh")}
        </button>
      </footer>
    </section>
  );
}
