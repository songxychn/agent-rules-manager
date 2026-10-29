// Linear in file size. Highlight the changed middle as chunks, keeping exact
// text (including trailing newlines) without allocating a DOM node per line.
export function splitSyncDiff(before: string | null, after: string | null) {
  const a = (before ?? "").match(/[^\n]*\n|[^\n]+$/g) ?? [];
  const b = (after ?? "").match(/[^\n]*\n|[^\n]+$/g) ?? [];
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) start++;
  let endA = a.length, endB = b.length;
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) { endA--; endB--; }
  return { prefix: a.slice(0, start).join(""), removed: a.slice(start, endA).join(""), added: b.slice(start, endB).join(""), suffix: a.slice(endA).join("") };
}
