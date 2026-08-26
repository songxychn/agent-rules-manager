import { useState } from "react";
import type { LibraryPlan, PackDraft, PackFileDraft, RulePackSummary } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { LibraryPlanPreview } from "./LibraryPlanPreview";

interface RulePacksPageProps {
  packs: RulePackSummary[];
  activePackIds: string[];
  legacySourcePath?: string;
  onOpen: (packId: string, path: string) => void;
  onPreviewCreate: (draft: PackDraft) => Promise<LibraryPlan>;
  onCreate: (draft: PackDraft) => Promise<void>;
  onPreviewAddFile: (draft: PackFileDraft) => Promise<LibraryPlan>;
  onAddFile: (draft: PackFileDraft) => Promise<void>;
}

const emptyDraft: PackDraft = { id: "", name: "", description: "" };

export function RulePacksPage({
  packs,
  activePackIds,
  legacySourcePath,
  onOpen,
  onPreviewCreate,
  onCreate,
  onPreviewAddFile,
  onAddFile,
}: RulePacksPageProps) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<PackDraft>(emptyDraft);
  const [plan, setPlan] = useState<LibraryPlan>();
  const [busy, setBusy] = useState<"preview" | "create">();
  const [error, setError] = useState<string>();
  const [fileDraft, setFileDraft] = useState<PackFileDraft>();
  const [filePlan, setFilePlan] = useState<LibraryPlan>();
  const [fileBusy, setFileBusy] = useState<"preview" | "create">();

  const updateDraft = (field: keyof PackDraft, value: string) => {
    setDraft((current) => ({ ...current, [field]: value }));
    setPlan(undefined);
    setError(undefined);
  };

  const preview = async () => {
    setBusy("preview");
    setError(undefined);
    try {
      setPlan(await onPreviewCreate(draft));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(undefined);
    }
  };

  const create = async () => {
    setBusy("create");
    setError(undefined);
    try {
      await onCreate(draft);
      setDraft(emptyDraft);
      setPlan(undefined);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(undefined);
    }
  };

  const previewFile = async () => {
    if (!fileDraft) return;
    setFileBusy("preview");
    setError(undefined);
    try {
      setFilePlan(await onPreviewAddFile(fileDraft));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setFileBusy(undefined);
    }
  };

  const addFile = async () => {
    if (!fileDraft) return;
    setFileBusy("create");
    setError(undefined);
    try {
      await onAddFile(fileDraft);
      setFileDraft(undefined);
      setFilePlan(undefined);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setFileBusy(undefined);
    }
  };

  return (
    <div className="library-page rule-packs-page">
      <header className="library-page-hero">
        <div>
          <p className="eyebrow">{t("packs.eyebrow")}</p>
          <h1>{t("packs.title")}</h1>
          <p>{t("packs.intro")}</p>
        </div>
        <div className="sync-boundary-ticket" role="note" aria-label={t("packs.sync.title")}>
          <span>{t("packs.sync.title")}</span>
          <div><strong>SYNC</strong><code>schema.json · packs/** · profiles/**</code></div>
          <div className="is-local"><strong>LOCAL</strong><code>.runtime/** · current · machine.json</code></div>
        </div>
      </header>

      {legacySourcePath && (
        <aside className="legacy-import-note">
          <strong>{t("packs.legacy.title")}</strong>
          <p>{t("packs.legacy.body")}</p>
          <code>{legacySourcePath}</code>
        </aside>
      )}

      <div className="library-page-grid">
        <section className="manifest-ledger" aria-label={t("packs.listLabel")}>
          <div className="manifest-ledger-heading">
            <span>{t("packs.ledger.pack")}</span>
            <span>{t("packs.ledger.sources")}</span>
            <span>{t("packs.ledger.state")}</span>
          </div>
          {packs.map((pack, packIndex) => {
            const active = activePackIds.includes(pack.id);
            return (
              <article className={`pack-manifest-row ${active ? "is-routed" : ""}`} key={pack.id}>
                <div className="pack-identity">
                  <span className="manifest-index">P{String(packIndex + 1).padStart(2, "0")}</span>
                  <div>
                    <span className="manifest-id">{pack.id}</span>
                    <h2>{pack.name}</h2>
                    <p>{pack.description || t("packs.noDescription")}</p>
                  </div>
                </div>

                <ol className="pack-file-list">
                  {pack.files.map((file, fileIndex) => (
                    <li key={file.path}>
                      <span className="file-order">{String(fileIndex + 1).padStart(2, "0")}</span>
                      <span className="file-route-line" aria-hidden="true" />
                      <div>
                        <strong>{file.path}</strong>
                        <code>{file.digest}</code>
                      </div>
                      {fileIndex === 0 && file.path === "AGENTS.md" && (
                        <small>{t("packs.required")}</small>
                      )}
                      <button
                        className="inline-action"
                        onClick={() => onOpen(pack.id, file.path)}
                      >
                        {t("packs.open")}
                      </button>
                    </li>
                  ))}
                </ol>

                <div className="pack-route-state">
                  <span className={active ? "route-lamp is-on" : "route-lamp"} aria-hidden="true" />
                  <strong>{active ? t("packs.routed") : t("packs.available")}</strong>
                  <small>{t("packs.fileCount", { count: pack.files.length })}</small>
                  <button
                    className="inline-action add-file-action"
                    onClick={() => {
                      setFileDraft({ packId: pack.id, relativePath: "rules/" });
                      setFilePlan(undefined);
                    }}
                  >
                    {t("packs.addFile")}
                  </button>
                </div>

                {fileDraft?.packId === pack.id && (
                  <div className="pack-file-composer">
                    <div>
                      <span>{t("packs.addFileTitle")}</span>
                      <code>{pack.id}/</code>
                    </div>
                    <input
                      autoFocus
                      value={fileDraft.relativePath}
                      aria-label={t("packs.addFilePath")}
                      onChange={(event) => {
                        setFileDraft({ ...fileDraft, relativePath: event.target.value });
                        setFilePlan(undefined);
                      }}
                    />
                    {!filePlan && (
                      <div className="pack-file-composer-actions">
                        <button className="button button-ghost button-small" onClick={() => setFileDraft(undefined)}>
                          {t("libraryPlan.cancel")}
                        </button>
                        <button
                          className="button button-primary button-small"
                          disabled={!fileDraft.relativePath || Boolean(fileBusy)}
                          onClick={() => void previewFile()}
                        >
                          {fileBusy === "preview" ? t("libraryPlan.previewing") : t("packs.addFilePreview")}
                        </button>
                      </div>
                    )}
                    {filePlan && (
                      <LibraryPlanPreview
                        plan={filePlan}
                        confirming={fileBusy === "create"}
                        confirmLabel={t("packs.addFileConfirm")}
                        onCancel={() => setFilePlan(undefined)}
                        onConfirm={() => void addFile()}
                      />
                    )}
                  </div>
                )}
              </article>
            );
          })}
        </section>

        <aside className="library-composer">
          <p className="eyebrow">{t("packs.create.eyebrow")}</p>
          <h2>{t("packs.create.title")}</h2>
          <p className="composer-intro">{t("packs.create.body")}</p>

          <label>
            <span>{t("packs.create.id")}</span>
            <input
              value={draft.id}
              placeholder="team-rules"
              onChange={(event) => updateDraft("id", event.target.value)}
            />
            <small>{t("packs.create.idHint")}</small>
          </label>
          <label>
            <span>{t("packs.create.name")}</span>
            <input
              value={draft.name}
              placeholder={t("packs.create.namePlaceholder")}
              onChange={(event) => updateDraft("name", event.target.value)}
            />
          </label>
          <label>
            <span>{t("packs.create.description")}</span>
            <textarea
              rows={3}
              value={draft.description}
              onChange={(event) => updateDraft("description", event.target.value)}
            />
          </label>

          <div className="required-entry-readout">
            <span>01</span>
            <div><strong>AGENTS.md</strong><small>{t("packs.create.entrypoint")}</small></div>
            <em>LOCKED</em>
          </div>

          {error && <p className="form-error" role="alert">{error}</p>}
          {!plan && (
            <button
              className="button button-primary composer-submit"
              disabled={!draft.id || !draft.name || Boolean(busy)}
              onClick={() => void preview()}
            >
              {busy === "preview" ? t("libraryPlan.previewing") : t("packs.create.preview")}
            </button>
          )}
          {plan && (
            <LibraryPlanPreview
              plan={plan}
              confirming={busy === "create"}
              confirmLabel={t("packs.create.confirm")}
              onCancel={() => setPlan(undefined)}
              onConfirm={() => void create()}
            />
          )}
        </aside>
      </div>
    </div>
  );
}
