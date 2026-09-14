import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "../lib/i18n";
import type { Notification, useNotifications } from "../lib/notifications";
import { AppIcon } from "./AppIcon";

function NotificationToast({ item, dismiss }: {
  item: Notification;
  dismiss: (id: string) => void;
}) {
  const { t } = useI18n();
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  useEffect(() => {
    if (hovered || focused) return;
    const timer = window.setTimeout(() => dismiss(item.id), 5000);
    return () => window.clearTimeout(timer);
  }, [item.id, dismiss, hovered, focused]);

  return (
    <article
      className={`notification-toast notification-${item.tone}`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setFocused(false);
      }}
    >
      <span className="notification-symbol" aria-hidden="true">
        {item.tone === "success" ? "✓" : item.tone === "error" ? "!" : "i"}
      </span>
      <div role={item.tone === "error" ? "alert" : "status"}>
        <strong>{t(`notifications.${item.tone}`)}</strong>
        <p>{item.message}</p>
      </div>
      <button className="notification-close" onClick={() => dismiss(item.id)} aria-label={t("notice.dismiss")}>×</button>
    </article>
  );
}

export function NotificationCenter({ history, pending, dismiss, markRead, clear }: ReturnType<typeof useNotifications>) {
  const { t, locale } = useI18n();
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const unread = history.filter((item) => !item.read).length;

  useEffect(() => {
    if (!open) return;
    closeRef.current?.focus();
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Node && !panelRef.current?.contains(event.target) &&
        !triggerRef.current?.contains(event.target)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);

  return (
    <>
      <button
        ref={triggerRef}
        className={`button button-ghost notification-trigger ${open ? "is-open" : ""}`}
        aria-label={unread ? t("notifications.unread", { count: unread }) : t("notifications.title")}
        title={t("notifications.title")}
        aria-expanded={open}
        aria-controls="notification-center"
        aria-haspopup="dialog"
        onClick={() => {
          if (!open) markRead();
          setOpen(!open);
        }}
      >
        <AppIcon name="bell" />
        {unread > 0 && <span className="notification-count" aria-hidden="true">{unread > 99 ? "99+" : unread}</span>}
      </button>
      {createPortal(
        <>
          {open && (
            <section ref={panelRef} id="notification-center" className="notification-center" role="dialog" aria-labelledby="notification-title">
              <div className="notification-header">
                <div><h2 id="notification-title">{t("notifications.title")}</h2><p>{t("notifications.retention")}</p></div>
                <button ref={closeRef} className="notification-close" aria-label={t("notifications.close")} onClick={() => {
                  setOpen(false);
                  triggerRef.current?.focus();
                }}>×</button>
              </div>
              <div className="notification-toolbar">
                <span>{t("notifications.total", { count: history.length })}</span>
                <button className="button button-ghost button-small" disabled={!unread} onClick={markRead}>{t("notifications.markRead")}</button>
                <button className="button button-ghost button-small" disabled={!history.length} onClick={clear}>{t("notifications.clear")}</button>
              </div>
              {history.length ? (
                <ol className="notification-history">
                  {history.map((item) => (
                    <li key={item.id} className={`notification-${item.tone} ${item.read ? "" : "is-unread"}`}>
                      <span className="notification-symbol" aria-hidden="true">{item.tone === "success" ? "✓" : item.tone === "error" ? "!" : "i"}</span>
                      <div>
                        <div className="notification-meta">
                          <strong>{t(`notifications.${item.tone}`)}</strong>
                          {!item.read && <span className="notification-unread-dot" title={t("notifications.new")} />}
                          <time dateTime={new Date(item.createdAt).toISOString()}>{new Date(item.createdAt).toLocaleString(locale, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" })}</time>
                        </div>
                        <p>{item.message}</p>
                      </div>
                    </li>
                  ))}
                </ol>
              ) : <div className="notification-empty"><AppIcon name="bell" /><strong>{t("notifications.empty")}</strong><p>{t("notifications.emptyHint")}</p></div>}
            </section>
          )}
          <aside className="notification-toasts" aria-label={t("notifications.new")}>
            {pending.slice(0, 3).map((item) => <NotificationToast key={item.id} item={item} dismiss={dismiss} />)}
          </aside>
        </>, document.body,
      )}
    </>
  );
}
