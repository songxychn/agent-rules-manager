import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type LanguagePreference = "system" | "zh-CN" | "en";
export type AppLocale = Exclude<LanguagePreference, "system">;

const STORAGE_KEY = "agent-rules-manager.language";

const en = {
  "brand.subtitle": "Local rules sync",
  "loading.inspecting": "Inspecting native rule paths…",
  "loading.failed": "Unable to load workspace.",
  "library.label": "Library",
  "library.load": "Load",
  "runtime.demo": "Browser demo",
  "rollback.latest": "Rollback latest",
  "rollback.restoring": "Restoring…",
  "nav.primary": "Primary navigation",
  "nav.control": "Control room",
  "nav.rulePacks": "Rule packs",
  "nav.profiles": "Profiles",
  "nav.settings": "Settings",
  "nav.next": "Next",
  "readout.paths": "Agent connections",
  "readout.aligned": "automatic sync enabled",
  "notice.dismiss": "Dismiss notification",
  "source.missing.eyebrow": "No canonical source",
  "source.missing.title": "Start with one neutral rules file.",
  "source.missing.body":
    "Agent Rules Manager will create {path}. No native agent path is touched until you inspect and confirm its connection.",
  "source.missing.initialize": "Initialize library",
  "notice.sourceOpened": "Opened AGENTS.md with {app}.",
  "notice.sourceRefreshed": "Rechecked the rules source and native paths.",
  "notice.demoOpen": "The browser demo cannot open local applications.",
  "notice.applied": "Connected safely: {agents}.",
  "notice.aligned": "Every agent is already connected.",
  "notice.restored": "Restored snapshot {id}.",
  "notice.initialized": "Created the neutral AGENTS.md source.",
  "projection.eyebrow": "Automatic sync",
  "projection.title": "One source, automatically available to every agent",
  "projection.hint": "Once connected, editing AGENTS.md needs no apply step.",
  "projection.canonical": "Rules source",
  "projection.uninitialized": "not initialized",
  "projection.targets": "Agent projection targets",
  "projection.managedInclude": "managed include",
  "projection.symbolicLink": "symbolic link",
  "projection.syncActiveTitle": "Automatic sync is active",
  "projection.syncActiveBody":
    "Connected agents read the same source whenever AGENTS.md changes.",
  "projection.attentionTitle": "{count} connection targets need attention",
  "projection.attentionBody":
    "Inspect the reference that will be created, then confirm before writing a native path.",
  "projection.conflictDetectedTitle": "{count} connection conflicts detected",
  "projection.conflictDetectedBody":
    "Inspect the conflict first. Unmanaged files are never overwritten.",
  "projection.blockedTitle": "Connection blocked",
  "projection.blockedBody":
    "An unmanaged file or unexpected link is present. Nothing was written.",
  "projection.connectionChangesReady": "Reviewed {count} connection changes",
  "projection.connectionChangesBody":
    "Review the highlighted agents. A rollback snapshot is created before confirmation.",
  "projection.inspect": "Inspect connection",
  "projection.inspecting": "Inspecting…",
  "projection.recheck": "Inspect again",
  "projection.applying": "Connecting…",
  "projection.connectOne": "Connect 1 target",
  "projection.connectMany": "Connect {count} targets",
  "projection.step.conflict": "An unmanaged file or unexpected link was detected.",
  "projection.step.addInclude":
    "Will preserve agent-specific rules and add a managed include.",
  "projection.step.createLink": "Will create a symbolic link to the rules source.",
  "projection.step.refreshManagedBlock": "Will refresh the existing managed include.",
  "projection.step.change": "Will establish the connection safely.",
  "source.file.eyebrow": "User-owned source",
  "source.file.title": "Edit rules in your own tools",
  "source.file.path": "rules source",
  "source.file.description":
    "Once saved, connected agents read the latest contents directly.",
  "source.file.digest": "Content digest",
  "source.file.modified": "Last modified",
  "source.file.unknownTime": "Not available",
  "source.file.externalTitle": "External editing boundary",
  "source.file.externalBody":
    "Save in your editor and return here. Connected agents read it directly; rollback never replaces it.",
  "source.file.watch": "Rechecks when this window regains focus",
  "source.file.refresh": "Recheck now",
  "source.file.refreshing": "Rechecking…",
  "source.open.action": "Open with",
  "source.open.with": "Open AGENTS.md with {app}",
  "source.open.default": "Default app",
  "source.open.unavailable": "No application available",
  "source.open.opening": "Opening…",
  "source.open.menu": "Choose how to open AGENTS.md",
  "source.open.editors": "Editors",
  "source.open.system": "System locations",
  "status.inSync": "Connected",
  "status.ready": "Not connected",
  "status.drifted": "Needs repair",
  "status.conflict": "Conflict",
  "settings.eyebrow": "Local preferences",
  "settings.title": "Settings",
  "settings.intro":
    "Choose how the control room speaks. This preference stays on this device and never changes your rule files.",
  "settings.language.eyebrow": "Interface",
  "settings.language.title": "Language",
  "settings.language.description": "Changes apply immediately across the application.",
  "settings.language.group": "Interface language",
  "settings.language.system": "Follow system",
  "settings.language.systemDescription": "Use the primary language reported by this device.",
  "settings.language.zh": "Simplified Chinese",
  "settings.language.zhDescription": "Always show the interface in Simplified Chinese.",
  "settings.language.en": "English",
  "settings.language.enDescription": "Always show the interface in English.",
  "settings.language.output": "Active output",
  "settings.language.systemDetected": "System resolves to {language}",
  "settings.safety.eyebrow": "Content boundary",
  "settings.safety.title": "Rules stay untouched",
  "settings.safety.body":
    "Language only affects application controls and messages. AGENTS.md content, paths, backups, and agent projections are never translated.",
  "settings.storage": "Saved locally in this WebView",
  "language.zh": "Simplified Chinese",
  "language.en": "English",
} as const;

type TranslationKey = keyof typeof en;
type TranslationVariables = Record<string, string | number>;

const zh: Record<TranslationKey, string> = {
  "brand.subtitle": "本地规则自动同步",
  "loading.inspecting": "正在检查 Agent 原生规则路径…",
  "loading.failed": "无法加载规则工作区。",
  "library.label": "规则库",
  "library.load": "加载",
  "runtime.demo": "浏览器演示",
  "rollback.latest": "回滚最近一次",
  "rollback.restoring": "正在恢复…",
  "nav.primary": "主导航",
  "nav.control": "控制台",
  "nav.rulePacks": "规则包",
  "nav.profiles": "配置方案",
  "nav.settings": "设置",
  "nav.next": "后续",
  "readout.paths": "Agent 接入",
  "readout.aligned": "已启用自动同步",
  "notice.dismiss": "关闭通知",
  "source.missing.eyebrow": "尚无规范源",
  "source.missing.title": "从一份中立规则开始。",
  "source.missing.body":
    "Agent Rules Manager 将创建 {path}。检查并确认接入之前，不会修改任何 Agent 原生文件。",
  "source.missing.initialize": "初始化规则库",
  "notice.sourceOpened": "已使用 {app} 打开 AGENTS.md。",
  "notice.sourceRefreshed": "已重新检查规则源和 Agent 原生路径。",
  "notice.demoOpen": "浏览器演示模式无法打开本地应用。",
  "notice.applied": "已安全完成接入：{agents}。",
  "notice.aligned": "所有 Agent 均已接入。",
  "notice.restored": "已恢复快照 {id}。",
  "notice.initialized": "已创建中立的 AGENTS.md 规则源。",
  "projection.eyebrow": "自动同步",
  "projection.title": "一份规则源，自动同步到各 Agent",
  "projection.hint": "接入完成后，修改 AGENTS.md 无需再次应用。",
  "projection.canonical": "规则源",
  "projection.uninitialized": "尚未初始化",
  "projection.targets": "Agent 投射目标",
  "projection.managedInclude": "托管引用",
  "projection.symbolicLink": "符号链接",
  "projection.syncActiveTitle": "自动同步已启用",
  "projection.syncActiveBody": "修改 AGENTS.md 后，已接入的 Agent 会直接读取同一份规则源。",
  "projection.attentionTitle": "{count} 个 Agent 尚未完成接入",
  "projection.attentionBody": "先检查将要建立的引用关系，确认后再写入 Agent 原生路径。",
  "projection.conflictDetectedTitle": "检测到 {count} 个接入冲突",
  "projection.conflictDetectedBody": "先检查冲突详情；程序不会覆盖未托管文件。",
  "projection.blockedTitle": "接入已阻止",
  "projection.blockedBody": "存在未托管文件或异常链接，未执行任何写入。",
  "projection.connectionChangesReady": "已检查 {count} 项接入调整",
  "projection.connectionChangesBody": "请核对高亮的 Agent；确认时会先创建回滚快照。",
  "projection.inspect": "检查接入",
  "projection.inspecting": "正在检查…",
  "projection.recheck": "重新检查",
  "projection.applying": "正在接入…",
  "projection.connectOne": "确认接入 1 项",
  "projection.connectMany": "确认接入 {count} 项",
  "projection.step.conflict": "检测到未托管文件或异常链接。",
  "projection.step.addInclude": "将保留 Agent 专属规则并添加托管引用。",
  "projection.step.createLink": "将创建指向规则源的符号链接。",
  "projection.step.refreshManagedBlock": "将刷新现有托管引用。",
  "projection.step.change": "将安全建立接入关系。",
  "source.file.eyebrow": "用户维护的规则源",
  "source.file.title": "使用你熟悉的工具编辑",
  "source.file.path": "规则源",
  "source.file.description": "保存后，已接入的 Agent 会直接读取最新内容。",
  "source.file.digest": "内容摘要",
  "source.file.modified": "最后修改",
  "source.file.unknownTime": "暂无记录",
  "source.file.externalTitle": "外部编辑边界",
  "source.file.externalBody":
    "在编辑器中保存后返回此处。已接入 Agent 会直接读取，回滚不会替换这份文件。",
  "source.file.watch": "窗口重新获得焦点时自动检查",
  "source.file.refresh": "立即检查",
  "source.file.refreshing": "正在检查…",
  "source.open.action": "打开方式",
  "source.open.with": "使用 {app} 打开 AGENTS.md",
  "source.open.default": "系统默认应用",
  "source.open.unavailable": "没有可用应用",
  "source.open.opening": "正在打开…",
  "source.open.menu": "选择 AGENTS.md 的打开方式",
  "source.open.editors": "编辑器",
  "source.open.system": "系统位置",
  "status.inSync": "已接入",
  "status.ready": "待接入",
  "status.drifted": "需修复",
  "status.conflict": "有冲突",
  "settings.eyebrow": "本地偏好",
  "settings.title": "设置",
  "settings.intro": "选择控制台使用的语言。该偏好只保存在本机，不会修改任何规则文件。",
  "settings.language.eyebrow": "界面",
  "settings.language.title": "语言",
  "settings.language.description": "切换后立即应用到整个应用。",
  "settings.language.group": "界面语言",
  "settings.language.system": "跟随系统",
  "settings.language.systemDescription": "使用当前设备报告的首选语言。",
  "settings.language.zh": "简体中文",
  "settings.language.zhDescription": "界面始终使用简体中文。",
  "settings.language.en": "English",
  "settings.language.enDescription": "界面始终使用 English。",
  "settings.language.output": "当前输出",
  "settings.language.systemDetected": "系统当前解析为{language}",
  "settings.safety.eyebrow": "内容边界",
  "settings.safety.title": "规则内容保持原样",
  "settings.safety.body":
    "语言设置只影响应用控件和提示。AGENTS.md 内容、路径、备份及 Agent 投射均不会被翻译。",
  "settings.storage": "偏好保存在当前 WebView 本地",
  "language.zh": "简体中文",
  "language.en": "English",
};

export function parseLanguagePreference(value: string | null): LanguagePreference {
  return value === "zh-CN" || value === "en" || value === "system" ? value : "system";
}

export function resolveSystemLocale(languages: readonly string[] = []): AppLocale {
  return languages[0]?.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export function resolveLocale(
  preference: LanguagePreference,
  languages: readonly string[] = [],
): AppLocale {
  return preference === "system" ? resolveSystemLocale(languages) : preference;
}

export function translate(
  locale: AppLocale,
  key: TranslationKey,
  variables: TranslationVariables = {},
): string {
  const template = (locale === "zh-CN" ? zh : en)[key];
  return Object.entries(variables).reduce(
    (message, [name, value]) => message.replaceAll(`{${name}}`, String(value)),
    template,
  );
}

interface I18nValue {
  preference: LanguagePreference;
  locale: AppLocale;
  systemLocale: AppLocale;
  setPreference: (preference: LanguagePreference) => void;
  t: (key: TranslationKey, variables?: TranslationVariables) => string;
}

const I18nContext = createContext<I18nValue | null>(null);

function browserLanguages(): readonly string[] {
  return navigator.languages.length ? navigator.languages : [navigator.language];
}

function storedPreference(): LanguagePreference {
  try {
    return parseLanguagePreference(window.localStorage.getItem(STORAGE_KEY));
  } catch {
    return "system";
  }
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<LanguagePreference>(storedPreference);
  const [systemLocale, setSystemLocale] = useState<AppLocale>(() =>
    resolveSystemLocale(browserLanguages()),
  );
  const locale = resolveLocale(preference, [systemLocale]);

  useEffect(() => {
    const updateSystemLocale = () => setSystemLocale(resolveSystemLocale(browserLanguages()));
    window.addEventListener("languagechange", updateSystemLocale);
    return () => window.removeEventListener("languagechange", updateSystemLocale);
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const setPreference = useCallback((next: LanguagePreference) => {
    setPreferenceState(next);
    try {
      window.localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // The preference still works for this session when storage is unavailable.
    }
  }, []);

  const t = useCallback(
    (key: TranslationKey, variables?: TranslationVariables) =>
      translate(locale, key, variables),
    [locale],
  );

  const value = useMemo(
    () => ({ preference, locale, systemLocale, setPreference, t }),
    [preference, locale, systemLocale, setPreference, t],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
}
