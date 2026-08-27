import {
  useEffect,
  useId,
  useRef,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { useI18n } from "../lib/i18n";

interface ConfirmDialogProps {
  title: string;
  description: string;
  tone?: "default" | "danger";
  blocked?: boolean;
  embedded?: boolean;
  confirming?: boolean;
  confirmLabel?: string;
  cancelLabel?: string;
  note?: string;
  children?: ReactNode;
  onConfirm?: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  title,
  description,
  tone = "default",
  blocked = false,
  embedded = false,
  confirming = false,
  confirmLabel,
  cancelLabel,
  note,
  children,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useI18n();
  const titleId = useId();
  const descriptionId = useId();
  const dialogRef = useRef<HTMLElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : undefined;
    const previousOverflow = document.body.style.overflow;
    if (!embedded) document.body.style.overflow = "hidden";
    const frame = window.requestAnimationFrame(() => cancelRef.current?.focus());

    return () => {
      window.cancelAnimationFrame(frame);
      if (!embedded) {
        document.body.style.overflow = previousOverflow;
        window.requestAnimationFrame(() => {
          if (previousFocus?.isConnected) previousFocus.focus();
        });
      }
    };
  }, [embedded]);

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (!confirming) onCancel();
      return;
    }
    if (embedded || event.key !== "Tab") return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), summary, input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ) ?? [],
    );
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  const dialog = (
    <section
      ref={dialogRef}
      className={`library-plan confirmation-dialog ${embedded ? "is-embedded" : ""} ${
        blocked ? "is-blocked" : ""
      } ${tone === "danger" ? "is-danger" : ""}`}
      role={embedded ? undefined : "dialog"}
      aria-modal={embedded ? undefined : true}
      aria-labelledby={titleId}
      aria-describedby={descriptionId}
      onKeyDown={handleKeyDown}
    >
      <div className="library-plan-heading">
        <span className="library-plan-stamp" aria-hidden="true">
          {blocked ? "!" : t("libraryPlan.confirmStamp")}
        </span>
        <div>
          <strong id={titleId}>{title}</strong>
          <p id={descriptionId}>{description}</p>
        </div>
      </div>

      {children}

      <footer>
        <span>{note}</span>
        <div>
          <button
            ref={cancelRef}
            type="button"
            className="button button-ghost button-small"
            disabled={confirming}
            onClick={onCancel}
          >
            {cancelLabel ?? t("libraryPlan.cancel")}
          </button>
          {confirmLabel && onConfirm && (
            <button
              type="button"
              className={`button ${tone === "danger" ? "button-danger" : "button-primary"} button-small`}
              disabled={blocked || confirming}
              onClick={onConfirm}
            >
              {confirming ? t("libraryPlan.applying") : confirmLabel}
            </button>
          )}
        </div>
      </footer>
    </section>
  );

  if (embedded) return dialog;
  return createPortal(
    <div
      className="confirmation-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget && !confirming) onCancel();
      }}
    >
      {dialog}
    </div>,
    document.body,
  );
}
