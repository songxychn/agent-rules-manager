import { useState } from "react";
import type {
  LibraryPlan,
  ProfileDraft,
  ProfileSummary,
  RulePackSummary,
  RuntimeState,
} from "../lib/types";
import { useI18n } from "../lib/i18n";
import { LibraryPlanPreview } from "./LibraryPlanPreview";

interface ProfilesPageProps {
  profiles: ProfileSummary[];
  packs: RulePackSummary[];
  runtimeState: RuntimeState;
  latestLibraryBackup?: string;
  onPreviewCreate: (draft: ProfileDraft) => Promise<LibraryPlan>;
  onCreate: (draft: ProfileDraft) => Promise<void>;
  onPreviewActivate: (profileId: string) => Promise<LibraryPlan>;
  onActivate: (profileId: string) => Promise<void>;
  onRollback: () => Promise<void>;
}

const emptyDraft: ProfileDraft = { id: "", name: "", description: "", packIds: [] };

export function ProfilesPage({
  profiles,
  packs,
  runtimeState,
  latestLibraryBackup,
  onPreviewCreate,
  onCreate,
  onPreviewActivate,
  onActivate,
  onRollback,
}: ProfilesPageProps) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<ProfileDraft>(emptyDraft);
  const [createPlan, setCreatePlan] = useState<LibraryPlan>();
  const [activation, setActivation] = useState<{ profileId: string; plan: LibraryPlan }>();
  const [busy, setBusy] = useState<string>();
  const [error, setError] = useState<string>();
  const [rollbackArmed, setRollbackArmed] = useState(false);

  const packById = new Map(packs.map((pack) => [pack.id, pack]));

  const updateDraft = (field: "id" | "name" | "description", value: string) => {
    setDraft((current) => ({ ...current, [field]: value }));
    setCreatePlan(undefined);
    setError(undefined);
  };

  const togglePack = (id: string) => {
    setDraft((current) => ({
      ...current,
      packIds: current.packIds.includes(id)
        ? current.packIds.filter((packId) => packId !== id)
        : [...current.packIds, id],
    }));
    setCreatePlan(undefined);
  };

  const movePack = (index: number, delta: number) => {
    setDraft((current) => {
      const target = index + delta;
      if (target < 0 || target >= current.packIds.length) return current;
      const packIds = [...current.packIds];
      [packIds[index], packIds[target]] = [packIds[target], packIds[index]];
      return { ...current, packIds };
    });
    setCreatePlan(undefined);
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
          <div><small>{t("profiles.machineLabel")}</small><strong>{t("profiles.machineLocal")}</strong></div>
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
            const pending = activation?.profileId === profile.id;
            return (
              <article className={`profile-route-card ${profile.isActive ? "is-active" : ""}`} key={profile.id}>
                <header>
                  <span className="profile-sequence">R{String(index + 1).padStart(2, "0")}</span>
                  <div>
                    <span className="manifest-id">{profile.id}</span>
                    <h2>{profile.name}</h2>
                    <p>{profile.description || t("profiles.noDescription")}</p>
                  </div>
                  {profile.isActive && <span className="active-machine-badge">{t("profiles.activeHere")}</span>}
                </header>

                <div className="profile-pack-route" role="group" aria-label={t("profiles.packOrder")}>
                  {profile.packIds.map((packId, packIndex) => (
                    <div className="profile-pack-port" key={packId}>
                      <span>{String(packIndex + 1).padStart(2, "0")}</span>
                      <strong>{packById.get(packId)?.name ?? packId}</strong>
                      <code>{packId}</code>
                      {packIndex < profile.packIds.length - 1 && <i aria-hidden="true">→</i>}
                    </div>
                  ))}
                  <div className="profile-output-port">
                    <span aria-hidden="true" />
                    <div><strong>current/</strong><code>AGENTS.md</code></div>
                  </div>
                </div>

                <footer>
                  <span className={`runtime-state runtime-${profile.isActive ? runtimeState : "missing"}`}>
                    {profile.isActive
                      ? t(`runtime.${runtimeState}` as "runtime.current")
                      : t("profiles.notSelected")}
                  </span>
                  <button
                    className={profile.isActive && runtimeState === "current" ? "button button-ghost button-small" : "button button-primary button-small"}
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

                {pending && (
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

          <fieldset className="pack-selector">
            <legend>{t("profiles.create.selectPacks")}</legend>
            {packs.map((pack) => (
              <label key={pack.id}>
                <input
                  type="checkbox"
                  checked={draft.packIds.includes(pack.id)}
                  onChange={() => togglePack(pack.id)}
                />
                <span><strong>{pack.name}</strong><code>{pack.id}</code></span>
              </label>
            ))}
          </fieldset>

          <div className="selected-pack-order">
            <span>{t("profiles.create.order")}</span>
            {draft.packIds.length === 0 && <p>{t("profiles.create.orderEmpty")}</p>}
            {draft.packIds.map((packId, index) => (
              <div key={packId}>
                <span>{String(index + 1).padStart(2, "0")}</span>
                <strong>{packById.get(packId)?.name ?? packId}</strong>
                <button disabled={index === 0} onClick={() => movePack(index, -1)} aria-label={t("profiles.create.moveUp")}>↑</button>
                <button disabled={index === draft.packIds.length - 1} onClick={() => movePack(index, 1)} aria-label={t("profiles.create.moveDown")}>↓</button>
              </div>
            ))}
          </div>

          {!createPlan && (
            <button
              className="button button-primary composer-submit"
              disabled={!draft.id || !draft.name || !draft.packIds.length || Boolean(busy)}
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
