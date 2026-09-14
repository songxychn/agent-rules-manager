import { describe, expect, it } from "vitest";
import { notificationReducer, parseNotifications, type Notification } from "./notifications";

const notice: Notification = { id: "one", tone: "error", message: "Could not load library", createdAt: 1, read: false };

describe("notification history", () => {
  it("keeps dismissed notifications unread and available after restoring history", () => {
    const added = notificationReducer({ history: [], pending: [] }, { type: "add", notification: notice });
    const dismissed = notificationReducer(added, { type: "dismiss", id: notice.id });
    expect(dismissed.pending).toEqual([]);
    expect(parseNotifications(JSON.stringify(dismissed.history))).toEqual([notice]);
  });

  it("marks existing notifications read while keeping later arrivals unread", () => {
    const read = notificationReducer({ history: [notice], pending: [notice] }, { type: "read" });
    const next = notificationReducer(read, { type: "add", notification: { ...notice, id: "two" } });
    expect(next.history.map((item) => item.read)).toEqual([false, true]);
    expect(next.pending).toHaveLength(2);
  });

  it("clears history and pending popups together", () => {
    expect(notificationReducer({ history: [notice], pending: [notice] }, { type: "clear" }))
      .toEqual({ history: [], pending: [] });
  });

  it("recovers from corrupt storage and rejects invalid timestamps and duplicate ids", () => {
    expect(parseNotifications("broken JSON")).toEqual([]);
    expect(parseNotifications('{"history":[]}')).toEqual([]);
    expect(parseNotifications(JSON.stringify([
      null, notice, notice, { ...notice, id: "bad-time", createdAt: 1e20 },
      { ...notice, id: "bad-tone", tone: "unknown" }, { ...notice, id: "bad-read", read: "false" },
    ]))).toEqual([notice]);
  });

  it("retains the newest 100 notifications", () => {
    const history = Array.from({ length: 105 }, (_, index) => ({ ...notice, id: String(index), createdAt: index }));
    const restored = parseNotifications(JSON.stringify(history));
    expect(restored).toHaveLength(100);
    expect(restored[0].id).toBe("104");
    expect(restored[99].id).toBe("5");
    const next = notificationReducer({ history: restored, pending: [] }, {
      type: "add", notification: { ...notice, id: "105", createdAt: 105 },
    });
    expect(next.history).toHaveLength(100);
    expect(next.history[99].id).toBe("6");
  });
});
