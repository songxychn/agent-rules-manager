import { useCallback, useEffect, useReducer } from "react";

export type Notice = { tone: "success" | "error" | "info"; message: string };
export type Notification = Notice & { id: string; createdAt: number; read: boolean };
export const NOTIFICATION_STORAGE_KEY = "agent-rules-manager.notifications.v1";
export const NOTIFICATION_LIMIT = 100;

export function parseNotifications(value: string | null): Notification[] {
  try {
    const parsed: unknown = JSON.parse(value ?? "[]");
    if (!Array.isArray(parsed)) return [];
    const seen = new Set<string>();
    return parsed.filter((item): item is Notification => {
      if (!item || typeof item !== "object" || typeof item.id !== "string" ||
        !item.id || seen.has(item.id) || typeof item.message !== "string" ||
        !["success", "error", "info"].includes(item.tone) ||
        typeof item.createdAt !== "number" || !Number.isFinite(item.createdAt) ||
        item.createdAt < 0 || item.createdAt > 8.64e15 || typeof item.read !== "boolean") return false;
      seen.add(item.id);
      return true;
    }).sort((a, b) => b.createdAt - a.createdAt).slice(0, NOTIFICATION_LIMIT);
  } catch {
    return [];
  }
}

export interface NotificationState {
  history: Notification[];
  pending: Notification[];
}

type Action =
  | { type: "add"; notification: Notification }
  | { type: "dismiss"; id: string }
  | { type: "read" }
  | { type: "clear" };

export function notificationReducer(state: NotificationState, action: Action): NotificationState {
  switch (action.type) {
    case "add":
      return {
        history: [action.notification, ...state.history].slice(0, NOTIFICATION_LIMIT),
        pending: [...state.pending, action.notification].slice(-NOTIFICATION_LIMIT),
      };
    case "dismiss":
      return { ...state, pending: state.pending.filter((item) => item.id !== action.id) };
    case "read":
      return { ...state, history: state.history.map((item) => ({ ...item, read: true })) };
    case "clear":
      return { history: [], pending: [] };
  }
}

function initialState(): NotificationState {
  try {
    return { history: parseNotifications(window.localStorage.getItem(NOTIFICATION_STORAGE_KEY)), pending: [] };
  } catch {
    return { history: [], pending: [] };
  }
}

export function useNotifications() {
  const [state, dispatch] = useReducer(notificationReducer, undefined, initialState);
  useEffect(() => {
    try {
      window.localStorage.setItem(NOTIFICATION_STORAGE_KEY, JSON.stringify(state.history));
    } catch {
      // Notifications remain available for this session when local storage is unavailable.
    }
  }, [state.history]);
  const notify = useCallback((notice: Notice) => {
    dispatch({ type: "add", notification: {
      ...notice, id: crypto.randomUUID(), createdAt: Date.now(), read: false,
    } });
  }, []);
  const dismiss = useCallback((id: string) => dispatch({ type: "dismiss", id }), []);
  const markRead = useCallback(() => dispatch({ type: "read" }), []);
  const clear = useCallback(() => dispatch({ type: "clear" }), []);
  return { ...state, notify, dismiss, markRead, clear };
}
