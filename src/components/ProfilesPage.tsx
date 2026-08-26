import { useState } from "react";
import type {
  LibraryPlan,
  ProfileDraft,
  ProfileFileDraft,
  ProfileSummary,
  RuntimeState,
} from "../lib/types";
import { useI18n } from "../lib/i18n";
import { LibraryPlanPreview } from "./LibraryPlanPreview";

interface ProfilesPageProps {
  profiles: ProfileSummary[];
  runtimeState: RuntimeState;
  latestLibraryBackup?: string;
  onOpen: (profileId: string, path: string) => void;
  onPreviewCreate: (draft: ProfileDraft) => Promise<LibraryPlan>;
  onCreate: (draft: ProfileDraft) => Promise<void>;
  onPreviewAddFile: (draft: ProfileFileDraft) => Promise<LibraryPlan>;
  onAddFile: (draft: ProfileFileDraft) => Promise<void>;
  onPreviewActivate: (profileId: string) => Promise<LibraryPlan>;
  onActivate: (profileId: string) => Promise<void>;
  onRollback: () => Promise<void>;
}

const emptyDraft: ProfileDraft = { id: "", name: "", description: "" };

export function ProfilesPage({
  profiles,
  runtimeState,
  latestLibraryBackup,
  onOpen,
  onPreviewCreate,
  onCreate,
  onPreviewAddFile,
  onAddFile,
  onPreviewActivate,
  onActivate,
  onRollback,
}: ProfilesPageProps) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<ProfileDraft>(emptyDraft);
  const [createPlan, setCreatePlan] = useState<LibraryPlan>();
  const [fileDraft, setFileDraft] = useState<ProfileFileDraft>();
  const [filePlan, setFilePlan] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [activation, setActivation] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [busy, setBusy] = useState<string>();
  const [error, setError] = useState<string>();
  const [rollbackArmed, setRollbackArmed] = useState(false);

  const updateDraft = (field: keyof ProfileDraft, value: string) => {
    setDraft((current) => ({ ...current, [field]: value }));
    setCreatePlan(undefined);
    setError(undefined);
  };

  const run = async (key: string, action: () => Promise<void>) => {
    setBusy(key);
    setError(undefined);
    try {
      await action();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(undefined);
    }
  };

  const previewCreate = () =>
    run("preview-create", async () => setCreatePlan(await onPreviewCreate(draft)));

  const create = () =>
    run("create", async () => {
      await onCreate(draft);
      setDraft(emptyDraft);
      setCreatePlan(undefined);
    });

  const previewFile = (nextDraft: ProfileFileDraft) =>
    run(`preview-file-${nextDraft.profileId}`, async () => {
      setFilePlan({
        profileId: nextDraft.profileId,
        plan: await onPreviewAddFile(nextDraft),
      });
    });

  const addFile = (nextDraft: ProfileFileDraft) =>
    run(`add-file-${nextDraft.profileId}`, async () => {
      await onAddFile(nextDraft);
      setFileDraft(undefined);
      setFilePlan(undefined);
    });

  const previewActivation = (profileId: string) =>
    run(`preview-${profileId}`, async () => {
      setActivation({ profileId, plan: await onPreviewActivate(profileId) });
    });

  const activate = (profileId: string) =>
    run(`activate-${profileId}`, async () => {
      await onActivate(profileId);
      setActivation(undefined);
    });

  const rollback = () =>
    run("rollback", async () => {
      await onRollback();
      setRollbackArmed(false);
    });

  return (
    <div className="library-page profiles-page">
      <header className="library-page-hero profiles-hero">
        <div>
          <p className="eyebrow">{t("profiles.eyebrow")}</p>
          <h1>{t("profiles.title")}</h1>
          <p>{t("profiles.intro")}</p>
        </div>
        <div className="machine-selection-readout">
          <span className="route-lamp is-on" aria-hidden="true" />
          <div>
            <small>{t("profiles.machineLabel")}</small>
            <strong>{t("profiles.machineLocal")}</strong>
          </div>
          <code>machine.json</code>
        </div>
      </header>

      <div className="profile-toolbar">
        <p>{t("profiles.syncNote")}</p>
        <div>
          {rollbackArmed ? (
            <div className="rollback-confirm">
              <span>{t("profiles.rollbackPreview", { id: latestLibraryBackup ?? "—" })}</span>
              <button className="button button-ghost button-small" onClick={() => setRollbackArmed(false)}>
                {t("libraryPlan.cancel")}
              </button>
              <button
                className="button button-secondary button-small"
                disabled={busy === "rollback"}
                onClick={() => void rollback()}
              >
                {busy === "rollback" ? t("rollback.restoring") : t("profiles.rollbackConfirm")}
              </button>
            </div>
          ) : (
            <button
              className="button button-ghost button-small"
              disabled={!latestLibraryBackup}
              onClick={() => setRollbackArmed(true)}
            >
              {t("profiles.rollback")}
            </button>
          )}
        </div>
      </div>

      {error && <p className="form-error page-error" role="alert">{error}</p>}

      <div className="library-page-grid">
        <section className="profile-patch-sheet" aria-label={t("profiles.listLabel")}>
          {profiles.map((profile, index) => {
            const activationPending = activation?.profileId === profile.id;
            const editingFile = fileDraft?.profileId === profile.id;
            const filePreviewPending = filePlan?.profileId === profile.id;

            return (
              <article className={`profile-route-card ${profile.isActive ? "is-active" : ""}`} key={profile.id}>
                <header>
                  <span className="profile-sequence">P{String(index + 1).padStart(2, "0")}</span>
                  <div>
                    <span className="manifest-id">{profile.id}</span>
                    <h2>{profile.name}</h2>
                    <p>{profile.description || t("profiles.noDescription")}</p>
                  </div>
                  {profile.isActive && <span className="active-machine-badge">{t("profiles.activeHere")}</span>}
                </header>

                <div className="profile-files-heading">
                  <span>{t("profiles.files")}</span>
                  <small>{t("profiles.fileCount", { count: profile.files.length })}</small>
                </div>
                <ol className="pack-file-list profile-file-list">
                  {profile.files.map((file, fileIndex) => (
                    <li key={file.path}>
                      <span className="file-order">{String(fileIndex + 1).padStart(2, "0")}</span>
                      <span className="file-route-line" aria-hidden="true" />
                      <div>
                        <strong>{file.path}</strong>
                        <code>profiles/{profile.id}/{file.path}</code>
                      </div>
                      <small>{fileIndex === 0 ? t("profiles.required") : file.digest.slice(0, 8)}</small>
                      <button className="inline-action" onClick={() => onOpen(profile.id, file.path)}>
                        {t("profiles.open")}
                      </button>
                    </li>
                  ))}
                </ol>

                <div className="profile-output-strip">
                  <span aria-hidden="true">{profile.files.map(() => "·").join(" ")}</span>
                  <strong>{t("profiles.output")}</strong>
                  <code>current/AGENTS.md</code>
                </div>

                {!editingFile && (
                  <button
                    className="inline-action add-file-action"
                    disabled={Boolean(busy)}
                    onClick={() => {
                      setFileDraft({ profileId: profile.id, relativePath: "rules/" });
                      setFilePlan(undefined);
                    }}
                  >
                    {t("profiles.addFile")}
                  </button>
                )}

                {editingFile && fileDraft && (
                  <div className="pack-file-composer profile-file-composer">
                    <div>
                      <span>{t("profiles.addFileTitle")}</span>
                      <code>profiles/{profile.id}/</code>
                    </div>
                    <input
                      aria-label={t("profiles.addFilePath")}
                      value={fileDraft.relativePath}
                      onChange={(event) => {
                        setFileDraft({ ...fileDraft, relativePath: event.target.value });
                        setFilePlan(undefined);
                      }}
                    />
                    {!filePreviewPending && (
                      <div className="pack-file-composer-actions">
                        <button
                          className="button button-ghost button-small"
                          onClick={() => setFileDraft(undefined)}
                        >
                          {t("libraryPlan.cancel")}
                        </button>
                        <button
                          className="button button-primary button-small"
                          disabled={!fileDraft.relativePath || Boolean(busy)}
                          onClick={() => void previewFile(fileDraft)}
                        >
                          {busy === `preview-file-${profile.id}`
                            ? t("libraryPlan.previewing")
                            : t("profiles.addFilePreview")}
                        </button>
                      </div>
                    )}
                    {filePreviewPending && (
                      <LibraryPlanPreview
                        plan={filePlan.plan}
                        confirming={busy === `add-file-${profile.id}`}
                        confirmLabel={t("profiles.addFileConfirm")}
                        onCancel={() => setFilePlan(undefined)}
                        onConfirm={() => void addFile(fileDraft)}
                      />
                    )}
                  </div>
                )}

                <footer>
                  <span className={`runtime-state runtime-${profile.isActive ? runtimeState : "missing"}`}>
                    {profile.isActive
                      ? t(`runtime.${runtimeState}` as "runtime.current")
                      : t("profiles.notSelected")}
                  </span>
                  <button
                    className={profile.isActive && runtimeState === "current"
                      ? "button button-ghost button-small"
                      : "button button-primary button-small"}
                    disabled={busy !== undefined}
                    onClick={() => void previewActivation(profile.id)}
                  >
                    {busy === `preview-${profile.id}`
                      ? t("libraryPlan.previewing")
                      : profile.isActive
                        ? t("profiles.refresh")
                        : t("profiles.previewSwitch")}
                  </button>
                </footer>

                {activationPending && (
                  <LibraryPlanPreview
                    plan={activation.plan}
                    confirming={busy === `activate-${profile.id}`}
                    confirmLabel={profile.isActive ? t("profiles.confirmRefresh") : t("profiles.confirmSwitch")}
                    onCancel={() => setActivation(undefined)}
                    onConfirm={() => void activate(profile.id)}
                  />
                )}
              </article>
            );
          })}
        </section>

        <aside className="library-composer profile-composer">
          <p className="eyebrow">{t("profiles.create.eyebrow")}</p>
          <h2>{t("profiles.create.title")}</h2>
          <p className="composer-intro">{t("profiles.create.body")}</p>

          <label>
            <span>{t("profiles.create.id")}</span>
            <input
              value={draft.id}
              placeholder="client-work"
              onChange={(event) => updateDraft("id", event.target.value)}
            />
          </label>
          <label>
            <span>{t("profiles.create.name")}</span>
            <input
              value={draft.name}
              placeholder={t("profiles.create.namePlaceholder")}
              onChange={(event) => updateDraft("name", event.target.value)}
            />
          </label>
          <label>
            <span>{t("profiles.create.description")}</span>
            <textarea
              rows={3}
              value={draft.description}
              onChange={(event) => updateDraft("description", event.target.value)}
            />
          </label>

          <div className="required-entry-readout">
            <span>01</span>
            <div>
              <strong>AGENTS.md</strong>
              <small>{t("profiles.create.entrypoint")}</small>
            </div>
            <em>{t("profiles.required")}</em>
          </div>

          {!createPlan && (
            <button
              className="button button-primary composer-submit"
              disabled={!draft.id || !draft.name || Boolean(busy)}
              onClick={() => void previewCreate()}
            >
              {busy === "preview-create" ? t("libraryPlan.previewing") : t("profiles.create.preview")}
            </button>
          )}
          {createPlan && (
            <LibraryPlanPreview
              plan={createPlan}
              confirming={busy === "create"}
              confirmLabel={t("profiles.create.confirm")}
              onCancel={() => setCreatePlan(undefined)}
              onConfirm={() => void create()}
            />
          )}
        </aside>
      </div>
    </div>
  );
}
