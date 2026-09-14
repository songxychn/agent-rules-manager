import brandMark from "../../assets/icon-source.svg?no-inline";

type AppIconName = "control" | "profiles" | "settings" | "folder" | "rollback" | "bell";

export function BrandMark() {
  return (
    <img className="logo-mark" src={brandMark} alt="" aria-hidden="true" />
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
      {name === "bell" && (
        <>
          <path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9Z" />
          <path d="M10 21h4" />
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
