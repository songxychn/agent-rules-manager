import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import type {
  LibraryPlan,
  OpenTarget,
  ProfileDraft,
  ProfileFileDraft,
  ProfileSummary,
  RuntimeState,
} from "../lib/types";
import { useI18n } from "../lib/i18n";
import { ConfirmDialog } from "./ConfirmDialog";
import { LibraryChangeConfirm } from "./LibraryChangeConfirm";
import { SourceOpenControl } from "./SourceOpenControl";

interface ProfilesPageProps {
  profiles: ProfileSummary[];
  runtimeState: RuntimeState;
  latestLibraryBackup?: string;
  openTargets: OpenTarget[];
  preferredOpenTargetId: string;
  openingTargetId?: string;
  onOpen: (profileId: string, path: string, targetId: string) => void;
  onPrepareCreate: (draft: ProfileDraft) => Promise<LibraryPlan>;
  onCreate: (draft: ProfileDraft) => Promise<void>;
  onPrepareAddFile: (draft: ProfileFileDraft) => Promise<LibraryPlan>;
  onAddFile: (draft: ProfileFileDraft) => Promise<void>;
  onPrepareRemoveFile: (draft: ProfileFileDraft) => Promise<LibraryPlan>;
  onRemoveFile: (draft: ProfileFileDraft) => Promise<void>;
  onPrepareDelete: (profileId: string) => Promise<LibraryPlan>;
  onDelete: (profileId: string) => Promise<void>;
  onPrepareActivate: (profileId: string) => Promise<LibraryPlan>;
  onActivate: (profileId: string) => Promise<void>;
  onRollback: () => Promise<void>;
}

const emptyDraft: ProfileDraft = { id: "", name: "", description: "" };

export function ProfilesPage({
  profiles,
  runtimeState,
  latestLibraryBackup,
  openTargets,
  preferredOpenTargetId,
  openingTargetId,
  onOpen,
  onPrepareCreate,
  onCreate,
  onPrepareAddFile,
  onAddFile,
  onPrepareRemoveFile,
  onRemoveFile,
  onPrepareDelete,
  onDelete,
  onPrepareActivate,
  onActivate,
  onRollback,
}: ProfilesPageProps) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<ProfileDraft>(emptyDraft);
  const [createPlan, setCreatePlan] = useState<LibraryPlan>();
  const [fileDraft, setFileDraft] = useState<ProfileFileDraft>();
  const [filePlan, setFilePlan] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [fileRemoval, setFileRemoval] = useState<{
    draft: ProfileFileDraft;
    plan: LibraryPlan;
  }>();
  const [activation, setActivation] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [deletion, setDeletion] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [busy, setBusy] = useState<string>();
  const [error, setError] = useState<string>();
  const [rollbackArmed, setRollbackArmed] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const createTriggerRef = useRef<HTMLButtonElement>(null);
  const createDialogRef = useRef<HTMLElement>(null);
  const createIdInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!createOpen) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const frame = window.requestAnimationFrame(() => createIdInputRef.current?.focus());
    return () => {
      window.cancelAnimationFrame(frame);
      document.body.style.overflow = previousOverflow;
    };
  }, [createOpen]);

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

  const requestCreate = () =>
    run("prepare-create", async () => setCreatePlan(await onPrepareCreate(draft)));

  const create = () =>
    run("create", async () => {
      await onCreate(draft);
      setDraft(emptyDraft);
      setCreatePlan(undefined);
      setCreateOpen(false);
      window.requestAnimationFrame(() => createTriggerRef.current?.focus());
    });

  const closeCreate = () => {
    if (busy === "prepare-create" || busy === "create") return;
    setCreateOpen(false);
    setDraft(emptyDraft);
    setCreatePlan(undefined);
    setError(undefined);
    window.requestAnimationFrame(() => createTriggerRef.current?.focus());
  };

  const handleCreateDialogKeyDown = (event: ReactKeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      closeCreate();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = Array.from(
      createDialogRef.current?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), textarea:not([disabled])',
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

  const requestFileAdd = (nextDraft: ProfileFileDraft) =>
    run(`prepare-file-${nextDraft.profileId}`, async () => {
      setActivation(undefined);
      setDeletion(undefined);
      setFileRemoval(undefined);
      setFilePlan({
        profileId: nextDraft.profileId,
        plan: await onPrepareAddFile(nextDraft),
      });
    });

  const addFile = (nextDraft: ProfileFileDraft) =>
    run(`add-file-${nextDraft.profileId}`, async () => {
      await onAddFile(nextDraft);
      setFileDraft(undefined);
      setFilePlan(undefined);
    });

  const requestFileRemoval = (nextDraft: ProfileFileDraft) =>
    run(`prepare-remove-file-${nextDraft.profileId}`, async () => {
      setActivation(undefined);
      setDeletion(undefined);
      setFileDraft(undefined);
      setFilePlan(undefined);
      setFileRemoval({
        draft: nextDraft,
        plan: await onPrepareRemoveFile(nextDraft),
      });
    });

  const removeFile = (nextDraft: ProfileFileDraft) =>
    run(`remove-file-${nextDraft.profileId}`, async () => {
      await onRemoveFile(nextDraft);
      setFileRemoval(undefined);
    });

  const requestActivation = (profileId: string) =>
    run(`prepare-activation-${profileId}`, async () => {
      setDeletion(undefined);
      setFileRemoval(undefined);
      setActivation({ profileId, plan: await onPrepareActivate(profileId) });
    });

  const activate = (profileId: string) =>
    run(`activate-${profileId}`, async () => {
      await onActivate(profileId);
      setActivation(undefined);
    });

  const requestDeletion = (profileId: string) =>
    run(`prepare-delete-${profileId}`, async () => {
      setActivation(undefined);
      setFileDraft(undefined);
      setFilePlan(undefined);
      setFileRemoval(undefined);
      setDeletion({ profileId, plan: await onPrepareDelete(profileId) });
    });

  const deleteProfile = (profileId: string) =>
    run(`delete-${profileId}`, async () => {
      await onDelete(profileId);
      setDeletion(undefined);
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
        <div className="profile-toolbar-actions">
          <button
            ref={createTriggerRef}
            className="button button-primary button-small"
            disabled={Boolean(busy)}
            onClick={() => {
              setError(undefined);
              setCreateOpen(true);
            }}
          >
            {t("profiles.create.title")}
          </button>
          <button
            className="button button-ghost button-small"
            disabled={!latestLibraryBackup || Boolean(busy)}
            onClick={() => setRollbackArmed(true)}
          >
            {t("profiles.rollback")}
          </button>
        </div>
      </div>

      {rollbackArmed && (
        <ConfirmDialog
          title={t("profiles.rollbackQuestion")}
          description={t("profiles.rollbackPrompt", { id: latestLibraryBackup ?? "—" })}
          confirming={busy === "rollback"}
          confirmLabel={t("profiles.rollbackConfirm")}
          note={t("profiles.rollbackNote")}
          onCancel={() => setRollbackArmed(false)}
          onConfirm={() => void rollback()}
        />
      )}

      {error && !createOpen && <p className="form-error page-error" role="alert">{error}</p>}

      <div className="library-page-grid">
        <section className="profile-patch-sheet" aria-label={t("profiles.listLabel")}>
          {profiles.map((profile, index) => {
            const activationPending = activation?.profileId === profile.id;
            const deletionPending = deletion?.profileId === profile.id;
            const editingFile = fileDraft?.profileId === profile.id;
            const fileConfirmationPending = filePlan?.profileId === profile.id;
            const fileRemovalPending = fileRemoval?.draft.profileId === profile.id;

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
                      <div className="profile-file-actions">
                        <SourceOpenControl
                          fileName={file.path}
                          actionLabel={t("profiles.open")}
                          targets={openTargets}
                          preferredTargetId={preferredOpenTargetId}
                          openingTargetId={openingTargetId}
                          onOpen={(targetId) => onOpen(profile.id, file.path, targetId)}
                        />
                        {fileIndex > 0 && (
                          <button
                            className="profile-source-delete"
                            aria-label={t("profiles.removeFileLabel", { path: file.path })}
                            title={t("profiles.removeFileLabel", { path: file.path })}
                            disabled={Boolean(busy)}
                            onClick={() => void requestFileRemoval({
                              profileId: profile.id,
                              relativePath: file.path,
                            })}
                          >
                            {t("profiles.removeFile")}
                          </button>
                        )}
                      </div>
                    </li>
                  ))}
                </ol>

                {fileRemovalPending && (
                  <LibraryChangeConfirm
                    plan={fileRemoval.plan}
                    tone="danger"
                    title={t("profiles.removeFileQuestion")}
                    description={t("profiles.removeFilePrompt", {
                      profile: profile.name,
                      path: fileRemoval.draft.relativePath,
                    })}
                    confirming={busy === `remove-file-${profile.id}`}
                    confirmLabel={t("profiles.removeFileConfirm")}
                    onCancel={() => setFileRemoval(undefined)}
                    onConfirm={() => void removeFile(fileRemoval.draft)}
                  />
                )}

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
                      setActivation(undefined);
                      setDeletion(undefined);
                      setFileRemoval(undefined);
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
                    {!fileConfirmationPending && (
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
                          onClick={() => void requestFileAdd(fileDraft)}
                        >
                          {busy === `prepare-file-${profile.id}`
                            ? t("libraryPlan.preparing")
                            : t("profiles.addFileSubmit")}
                        </button>
                      </div>
                    )}
                    {fileConfirmationPending && (
                      <LibraryChangeConfirm
                        plan={filePlan.plan}
                        title={t("profiles.addFileQuestion")}
                        description={t("profiles.addFilePrompt", {
                          profile: profile.name,
                          path: fileDraft.relativePath,
                        })}
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
                  <div className="profile-card-actions">
                    <button
                      className="button button-danger-ghost button-small"
                      disabled={profile.isActive || busy !== undefined}
                      title={profile.isActive ? t("profiles.deleteActiveTitle") : undefined}
                      aria-label={profile.isActive ? t("profiles.deleteActiveTitle") : undefined}
                      onClick={() => void requestDeletion(profile.id)}
                    >
                      {busy === `prepare-delete-${profile.id}`
                        ? t("libraryPlan.preparing")
                        : t("profiles.delete")}
                    </button>
                    <button
                      className={profile.isActive && runtimeState === "current"
                        ? "button button-ghost button-small"
                        : "button button-primary button-small"}
                      disabled={busy !== undefined}
                      onClick={() => void requestActivation(profile.id)}
                    >
                      {busy === `prepare-activation-${profile.id}`
                        ? t("libraryPlan.preparing")
                        : profile.isActive
                          ? t("profiles.refresh")
                          : t("profiles.switch")}
                    </button>
                  </div>
                </footer>

                {activationPending && (
                  <LibraryChangeConfirm
                    plan={activation.plan}
                    title={profile.isActive
                      ? t("profiles.refreshQuestion")
                      : t("profiles.switchQuestion")}
                    description={profile.isActive
                      ? t("profiles.refreshPrompt", { profile: profile.name })
                      : t("profiles.switchPrompt", { profile: profile.name })}
                    confirming={busy === `activate-${profile.id}`}
                    confirmLabel={profile.isActive ? t("profiles.confirmRefresh") : t("profiles.confirmSwitch")}
                    onCancel={() => setActivation(undefined)}
                    onConfirm={() => void activate(profile.id)}
                  />
                )}

                {deletionPending && (
                  <LibraryChangeConfirm
                    plan={deletion.plan}
                    tone="danger"
                    title={t("profiles.deleteQuestion", { profile: profile.name })}
                    description={t("profiles.deletePrompt", {
                      profile: profile.name,
                      count: profile.files.length,
                    })}
                    confirming={busy === `delete-${profile.id}`}
                    confirmLabel={t("profiles.deleteConfirm")}
                    onCancel={() => setDeletion(undefined)}
                    onConfirm={() => void deleteProfile(profile.id)}
                  />
                )}
              </article>
            );
          })}
        </section>
      </div>

      {createOpen && (
        <div
          className="profile-create-backdrop"
          onPointerDown={(event) => {
            if (event.target === event.currentTarget) closeCreate();
          }}
        >
          <section
            ref={createDialogRef}
            className="library-composer profile-composer profile-create-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="profile-create-title"
            onKeyDown={handleCreateDialogKeyDown}
          >
            <button
              className="profile-create-close"
              aria-label={t("profiles.create.close")}
              disabled={busy === "prepare-create" || busy === "create"}
              onClick={closeCreate}
            >
              ×
            </button>
            <p className="eyebrow">{t("profiles.create.eyebrow")}</p>
            <h2 id="profile-create-title">{t("profiles.create.title")}</h2>
            {error && <p className="form-error" role="alert">{error}</p>}
            {!createPlan && (
              <>
                <p className="composer-intro">{t("profiles.create.body")}</p>

                <label>
                  <span>{t("profiles.create.id")}</span>
                  <input
                    ref={createIdInputRef}
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

                <button
                  className="button button-primary composer-submit"
                  disabled={!draft.id || !draft.name || Boolean(busy)}
                  onClick={() => void requestCreate()}
                >
                  {busy === "prepare-create"
                    ? t("libraryPlan.preparing")
                    : t("profiles.create.submit")}
                </button>
              </>
            )}
            {createPlan && (
              <LibraryChangeConfirm
                plan={createPlan}
                embedded
                title={t("profiles.create.question")}
                description={t("profiles.create.prompt", { id: draft.id, name: draft.name })}
                confirming={busy === "create"}
                confirmLabel={t("profiles.create.confirm")}
                onCancel={() => {
                  setCreatePlan(undefined);
                  window.requestAnimationFrame(() => createIdInputRef.current?.focus());
                }}
                onConfirm={() => void create()}
              />
            )}
          </section>
        </div>
      )}
    </div>
  );
}
