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
  ProfileSummary,
  RuntimeState,
} from "../lib/types";
import { useI18n } from "../lib/i18n";
import { LibraryChangeConfirm } from "./LibraryChangeConfirm";
import { SourceOpenControl } from "./SourceOpenControl";

interface ProfilesPageProps {
  profiles: ProfileSummary[];
  runtimeState: RuntimeState;
  openTargets: OpenTarget[];
  preferredOpenTargetId: string;
  openingTargetId?: string;
  onOpen: (profileId: string, path: string, targetId: string) => void;
  onPrepareCreate: (draft: ProfileDraft) => Promise<LibraryPlan>;
  onCreate: (draft: ProfileDraft) => Promise<void>;
  onPrepareDelete: (profileId: string) => Promise<LibraryPlan>;
  onDelete: (profileId: string) => Promise<void>;
  onPrepareActivate: (profileId: string) => Promise<LibraryPlan>;
  onActivate: (profileId: string) => Promise<void>;
  onHistory: () => void;
  onError: (message: string) => void;
}

const emptyDraft: ProfileDraft = { id: "", name: "", description: "" };

export function ProfilesPage({
  profiles,
  runtimeState,
  openTargets,
  preferredOpenTargetId,
  openingTargetId,
  onOpen,
  onPrepareCreate,
  onCreate,
  onPrepareDelete,
  onDelete,
  onPrepareActivate,
  onActivate,
  onHistory,
  onError,
}: ProfilesPageProps) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<ProfileDraft>(emptyDraft);
  const [createPlan, setCreatePlan] = useState<LibraryPlan>();
  const [activation, setActivation] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [deletion, setDeletion] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [busy, setBusy] = useState<string>();
  const [error, setError] = useState<string>();
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
      onError(String(cause));
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

  const requestActivation = (profileId: string) =>
    run(`prepare-activation-${profileId}`, async () => {
      setDeletion(undefined);
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
      setDeletion({ profileId, plan: await onPrepareDelete(profileId) });
    });

  const deleteProfile = (profileId: string) =>
    run(`delete-${profileId}`, async () => {
      await onDelete(profileId);
      setDeletion(undefined);
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
            disabled={Boolean(busy)}
            onClick={onHistory}
          >
            {t("nav.history")}
          </button>
        </div>
      </div>

      <div className="library-page-grid">
        <section className="profile-patch-sheet" aria-label={t("profiles.listLabel")}>
          {profiles.map((profile, index) => {
            const activationPending = activation?.profileId === profile.id;
            const deletionPending = deletion?.profileId === profile.id;

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

                <div className="profile-rule-file">
                  <div>
                    <strong>AGENTS.md</strong>
                    <code>profiles/{profile.id}/AGENTS.md</code>
                  </div>
                  <SourceOpenControl
                    fileName="AGENTS.md"
                    actionLabel={t("profiles.open")}
                    targets={openTargets}
                    preferredTargetId={preferredOpenTargetId}
                    openingTargetId={openingTargetId}
                    onOpen={(targetId) => onOpen(profile.id, "AGENTS.md", targetId)}
                  />
                </div>

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
                  <div>
                    <strong>AGENTS.md</strong>
                    <small>{t("profiles.create.entrypoint")}</small>
                  </div>
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
