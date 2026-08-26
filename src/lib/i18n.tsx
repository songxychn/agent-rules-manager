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
  "rollback.tooltip":
    "Restore the native Agent paths changed by the latest connection. The canonical AGENTS.md is not modified, and rollback stops if a target changed afterward.",
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
  "notice.demoOpen": "The browser demo cannot open local applications.",
  "notice.applied": "Applied connection choices: {agents}.",
  "notice.aligned": "The selected connection choices are already active.",
  "notice.noConnectionChanges": "Choose at least one agent to connect or make independent.",
  "notice.restored": "Restored snapshot {id}.",
  "notice.initialized": "Created the neutral AGENTS.md source.",
  "projection.eyebrow": "Agent discovery",
  "projection.title": "Choose which installed agents share the central rules",
  "projection.hint": "Connected agents use a symbolic link; independent agents keep their own file.",
  "projection.canonical": "Rules source",
  "projection.uninitialized": "not initialized",
  "projection.targets": "Agent projection targets",
  "projection.managedInclude": "managed include",
  "projection.symbolicLink": "symbolic link",
  "projection.detected": "Detected",
  "projection.notDetected": "Not detected on this computer",
  "projection.centralLink": "Central file · symbolic link",
  "projection.independentFile": "Independent native file",
  "projection.independentEmpty": "Independent · no native file yet",
  "projection.detail.missing": "No native rules file exists yet.",
  "projection.detail.connectedLink": "Reads the central rules file directly.",
  "projection.detail.independentFile": "Keeps rules in its own native file.",
  "projection.detail.legacyInclude": "Uses a legacy managed include that can be detached safely.",
  "projection.detail.foreignLink": "The existing symbolic link points somewhere else.",
  "projection.detail.invalidManagedFile": "The legacy managed markers are malformed.",
  "projection.toggleLabel": "Use central rules for {agent}",
  "projection.joined": "Central",
  "projection.independent": "Independent",
  "projection.detectedTitle": "{detected} agents detected · {connected} connected",
  "projection.detectedBody":
    "Each detected agent can stay independent or read the central file through a symbolic link.",
  "projection.selectionPendingTitle": "{count} connection choices are waiting for review",
  "projection.selectionPendingBody": "Inspect the exact native-path changes before applying them.",
  "projection.noneDetectedTitle": "No supported agents detected",
  "projection.noneDetectedBody":
    "Install or launch a supported agent, then recheck this workspace.",
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
    "An independent file or unexpected link is present. Nothing was overwritten.",
  "projection.connectionChangesReady": "Reviewed {count} connection changes",
  "projection.connectionChangesBody":
    "Review the highlighted agents. A rollback snapshot is created before confirmation.",
  "projection.inspect": "Save changes",
  "projection.inspecting": "Inspecting…",
  "projection.recheck": "Inspect again",
  "projection.applying": "Applying…",
  "projection.applyOne": "Apply 1 change",
  "projection.applyMany": "Apply {count} changes",
  "projection.connectOne": "Connect 1 target",
  "projection.connectMany": "Connect {count} targets",
  "projection.step.conflict": "An unmanaged file or unexpected link was detected.",
  "projection.step.addInclude":
    "Will preserve agent-specific rules and add a managed include.",
  "projection.step.createLink": "Will create a symbolic link to the rules source.",
  "projection.step.createIndependentFile":
    "Will replace the managed link with an independent copy of the current rules.",
  "projection.step.detachManagedInclude":
    "Will replace the legacy managed include with independent content.",
  "projection.step.migrateLegacyInclude":
    "Will migrate the legacy managed include to a symbolic link.",
  "projection.step.refreshManagedBlock": "Will refresh the existing managed include.",
  "projection.step.change": "Will establish the connection safely.",
  "source.open.with": "Open AGENTS.md with {app}",
  "source.open.default": "Default app",
  "source.open.unavailable": "No application available",
  "source.open.opening": "Opening…",
  "source.open.menu": "Choose how to open AGENTS.md",
  "status.inSync": "Connected",
  "status.ready": "Independent",
  "status.drifted": "Legacy link",
  "status.conflict": "Conflict",
  "status.unavailable": "Not installed",
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
  "rollback.tooltip":
    "恢复最近一次接入所修改的 Agent 原生路径，不会修改规则源 AGENTS.md；若目标此后发生变化，回滚将停止。",
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
  "notice.demoOpen": "浏览器演示模式无法打开本地应用。",
  "notice.applied": "已应用接入选择：{agents}。",
  "notice.aligned": "所选接入状态已生效。",
  "notice.noConnectionChanges": "请先选择至少一个 Agent 接入中央规则或改为独立使用。",
  "notice.restored": "已恢复快照 {id}。",
  "notice.initialized": "已创建中立的 AGENTS.md 规则源。",
  "projection.eyebrow": "Agent 检测",
  "projection.title": "选择哪些已安装 Agent 使用中央规则",
  "projection.hint": "接入后使用符号链接；不接入时保留 Agent 自己的独立文件。",
  "projection.canonical": "规则源",
  "projection.uninitialized": "尚未初始化",
  "projection.targets": "Agent 投射目标",
  "projection.managedInclude": "托管引用",
  "projection.symbolicLink": "符号链接",
  "projection.detected": "已检测",
  "projection.notDetected": "这台电脑上未检测到",
  "projection.centralLink": "中央文件 · 符号链接",
  "projection.independentFile": "独立原生文件",
  "projection.independentEmpty": "独立使用 · 尚无原生文件",
  "projection.detail.missing": "尚未创建 Agent 原生规则文件。",
  "projection.detail.connectedLink": "直接读取中央规则文件。",
  "projection.detail.independentFile": "继续使用自己的原生规则文件。",
  "projection.detail.legacyInclude": "仍在使用旧版托管引用，可安全退出接入。",
  "projection.detail.foreignLink": "现有符号链接指向了其他位置。",
  "projection.detail.invalidManagedFile": "旧版托管标记不完整或顺序异常。",
  "projection.toggleLabel": "让 {agent} 使用中央规则",
  "projection.joined": "接入",
  "projection.independent": "独立",
  "projection.detectedTitle": "检测到 {detected} 个 Agent · 已接入 {connected} 个",
  "projection.detectedBody": "每个 Agent 都可以独立持有规则，也可以通过符号链接读取中央文件。",
  "projection.selectionPendingTitle": "有 {count} 项接入选择等待检查",
  "projection.selectionPendingBody": "应用前先检查将要修改的 Agent 原生路径。",
  "projection.noneDetectedTitle": "未检测到受支持的 Agent",
  "projection.noneDetectedBody": "安装或启动受支持的 Agent 后，再重新检查当前工作区。",
  "projection.syncActiveTitle": "自动同步已启用",
  "projection.syncActiveBody": "修改 AGENTS.md 后，已接入的 Agent 会直接读取同一份规则源。",
  "projection.attentionTitle": "{count} 个 Agent 尚未完成接入",
  "projection.attentionBody": "先检查将要建立的引用关系，确认后再写入 Agent 原生路径。",
  "projection.conflictDetectedTitle": "检测到 {count} 个接入冲突",
  "projection.conflictDetectedBody": "先检查冲突详情；程序不会覆盖未托管文件。",
  "projection.blockedTitle": "接入已阻止",
  "projection.blockedBody": "目标已有独立文件或异常链接；为避免覆盖，未执行任何写入。",
  "projection.connectionChangesReady": "已检查 {count} 项接入调整",
  "projection.connectionChangesBody": "请核对高亮的 Agent；确认时会先创建回滚快照。",
  "projection.inspect": "保存修改",
  "projection.inspecting": "正在检查…",
  "projection.recheck": "重新检查",
  "projection.applying": "正在应用…",
  "projection.applyOne": "应用 1 项调整",
  "projection.applyMany": "应用 {count} 项调整",
  "projection.connectOne": "确认接入 1 项",
  "projection.connectMany": "确认接入 {count} 项",
  "projection.step.conflict": "检测到未托管文件或异常链接。",
  "projection.step.addInclude": "将保留 Agent 专属规则并添加托管引用。",
  "projection.step.createLink": "将创建指向规则源的符号链接。",
  "projection.step.createIndependentFile": "将把托管软链接替换为当前规则的独立副本。",
  "projection.step.detachManagedInclude": "将把旧版托管引用替换为独立规则内容。",
  "projection.step.migrateLegacyInclude": "将把旧版托管引用迁移为符号链接。",
  "projection.step.refreshManagedBlock": "将刷新现有托管引用。",
  "projection.step.change": "将安全建立接入关系。",
  "source.open.with": "使用 {app} 打开 AGENTS.md",
  "source.open.default": "系统默认应用",
  "source.open.unavailable": "没有可用应用",
  "source.open.opening": "正在打开…",
  "source.open.menu": "选择 AGENTS.md 的打开方式",
  "status.inSync": "已接入",
  "status.ready": "独立使用",
  "status.drifted": "旧版接入",
  "status.conflict": "有冲突",
  "status.unavailable": "未安装",
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
