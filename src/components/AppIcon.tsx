type AppIconName = "control" | "profiles" | "settings" | "folder" | "rollback";

export function BrandMark() {
  return (
    <svg className="logo-mark" viewBox="0 0 40 40" aria-hidden="true">
      <path className="logo-mark-back" d="M6.5 10.5A3.5 3.5 0 0 1 10 7h17.5A3.5 3.5 0 0 1 31 10.5V28a3.5 3.5 0 0 1-3.5 3.5H10A3.5 3.5 0 0 1 6.5 28Z" />
      <path className="logo-mark-front" d="M12 6.5h18A3.5 3.5 0 0 1 33.5 10v20A3.5 3.5 0 0 1 30 33.5H12A3.5 3.5 0 0 1 8.5 30V10A3.5 3.5 0 0 1 12 6.5Z" />
      <path className="logo-mark-route" d="M14 14h10.5M14 20h7m-7 6h11" />
      <circle className="logo-mark-port" cx="27.5" cy="20" r="2.5" />
    </svg>
  );
}

export function AppIcon({ name }: { name: AppIconName }) {
  return (
    <svg className="app-icon" viewBox="0 0 24 24" aria-hidden="true">
      {name === "control" && (
        <>
          <rect x="4" y="4" width="6" height="6" rx="1.5" />
          <rect x="14" y="4" width="6" height="6" rx="1.5" />
          <rect x="4" y="14" width="6" height="6" rx="1.5" />
          <rect x="14" y="14" width="6" height="6" rx="1.5" />
        </>
      )}
      {name === "profiles" && (
        <>
          <path d="M6 3.5h9.5L19 7v13.5H6Z" />
          <path d="M15 3.5V7h4M9 11h7M9 15h7" />
        </>
      )}
      {name === "settings" && (
        <>
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.86 2.86-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21h-4v-.1A1.7 1.7 0 0 0 8.6 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.86-2.86.06-.06A1.7 1.7 0 0 0 4.2 15a1.7 1.7 0 0 0-1.6-1H2.5v-4h.1A1.7 1.7 0 0 0 4.2 9a1.7 1.7 0 0 0-.34-1.88l-.06-.06L6.66 4.2l.06.06A1.7 1.7 0 0 0 8.6 4.6a1.7 1.7 0 0 0 1-1.6v-.1h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.88-.34l.06-.06 2.86 2.86-.06.06A1.7 1.7 0 0 0 19 9a1.7 1.7 0 0 0 1.6 1h.1v4h-.1a1.7 1.7 0 0 0-1.2 1Z" />
        </>
      )}
      {name === "folder" && <path d="M3.5 7.5h6l2-2h9v13h-17Z" />}
      {name === "rollback" && (
        <>
          <path d="M8 7H4v-4" />
          <path d="M4.5 7.5A8 8 0 1 1 4 15" />
        </>
      )}
    </svg>
  );
}
