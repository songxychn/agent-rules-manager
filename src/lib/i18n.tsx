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
  "brand.subtitle": "Local rule routing",
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
  "readout.aligned": "stable current entrypoint",
  "notice.dismiss": "Dismiss notification",
  "source.missing.eyebrow": "No canonical source",
  "source.missing.title": "Start with one neutral rules file.",
  "source.missing.body":
    "Agent Rules Manager will create {path}. No native agent path is touched until you inspect and confirm its connection.",
  "source.missing.initialize": "Initialize library",
  "notice.sourceOpened": "Opened {file} with {app}.",
  "notice.sourceRefreshed": "Rechecked Rule Pack sources, runtime, and native paths.",
  "notice.demoOpen": "The browser demo cannot open local applications.",
  "notice.applied": "Applied connection choices: {agents}.",
  "notice.aligned": "The selected connection choices are already active.",
  "notice.noConnectionChanges": "Choose at least one agent to connect or make independent.",
  "notice.restored": "Restored snapshot {id}.",
  "notice.initialized": "Created the Rule Pack library and selected its default Profile.",
  "notice.packCreated": "Created Rule Pack {name}.",
  "notice.packFileAdded": "Added {path}; refresh an active Profile that uses this Pack.",
  "notice.profileCreated": "Created Profile {name}.",
  "notice.profileActivated": "Profile {id} is now active on this machine.",
  "notice.libraryRestored": "Restored library snapshot {id}.",
  "projection.eyebrow": "Native projection",
  "projection.title": "One stable entrypoint, routed into every agent",
  "projection.hint": "Profiles can switch behind current/AGENTS.md without reconnecting agents.",
  "projection.canonical": "Active Profile",
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
  "projection.detail.legacyLink": "Still points to the migrated root rules path.",
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
  "projection.syncActiveTitle": "Native connections are aligned",
  "projection.syncActiveBody":
    "Connected agents resolve the machine-local current Profile through one stable path.",
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
  "projection.step.createLink": "Will create a symbolic link to current/AGENTS.md.",
  "projection.step.createIndependentFile":
    "Will replace the managed link with an independent copy of the current rules.",
  "projection.step.detachManagedInclude":
    "Will replace the legacy managed include with independent content.",
  "projection.step.migrateLegacyInclude":
    "Will migrate the legacy managed include to a symbolic link.",
  "projection.step.refreshManagedBlock": "Will refresh the existing managed include.",
  "projection.step.replaceLegacyLink": "Will move the managed legacy link to current/AGENTS.md.",
  "projection.step.change": "Will establish the connection safely.",
  "source.file.eyebrow": "Active Pack source",
  "source.file.title": "Edit source rules in your own tools",
  "source.file.path": "editable source",
  "source.file.description":
    "After saving, return here and refresh the active Profile runtime.",
  "source.file.digest": "Content digest",
  "source.file.modified": "Last modified",
  "source.file.unknownTime": "Not available",
  "source.file.externalTitle": "External editing boundary",
  "source.file.externalBody":
    "Rule Pack files are user-owned and syncable. Generated runtime and rollback never replace them.",
  "source.file.watch": "Rechecks source digests when this window regains focus",
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
  "libraryPlan.ready": "Mutation preview ready",
  "libraryPlan.blocked": "Mutation blocked",
  "libraryPlan.changeCount": "{count} filesystem change(s)",
  "libraryPlan.cancel": "Cancel",
  "libraryPlan.applying": "Applying…",
  "libraryPlan.previewing": "Preparing preview…",
  "setup.emptyEyebrow": "Empty library",
  "setup.emptyTitle": "Build the first Rule Pack route.",
  "setup.legacyEyebrow": "Legacy source found",
  "setup.legacyTitle": "Migrate AGENTS.md with a rollback snapshot.",
  "setup.conflictEyebrow": "Library conflict",
  "setup.conflictTitle": "This directory cannot be initialized safely.",
  "setup.preview": "Preview initialization",
  "setup.confirmCreate": "Create library",
  "setup.confirmImport": "Migrate legacy file",
  "runtimeGate.eyebrow": "Machine runtime",
  "runtimeGate.selectTitle": "Choose a Profile for this machine.",
  "runtimeGate.refreshTitle": "The active Profile needs a fresh runtime.",
  "runtimeGate.openProfiles": "Open Profiles",
  "runtime.current": "Runtime current",
  "runtime.missing": "Runtime missing",
  "runtime.stale": "Sources changed",
  "runtime.conflict": "Runtime conflict",
  "packs.eyebrow": "Versioned instruction inventory",
  "packs.title": "Rule Packs",
  "packs.intro":
    "Each Pack owns a required AGENTS.md plus optional ordered Markdown sources. Packs coexist; Profiles decide how they are composed.",
  "packs.sync.title": "Multi-machine boundary",
  "packs.legacy.title": "Unexpected root entry detected",
  "packs.legacy.body":
    "This ready library routes Pack sources through current/AGENTS.md. The root AGENTS.md is outside that route and will not be overwritten automatically.",
  "packs.listLabel": "Rule Pack manifests",
  "packs.ledger.pack": "Pack manifest",
  "packs.ledger.sources": "Ordered instruction sources",
  "packs.ledger.state": "Active route",
  "packs.noDescription": "No description",
  "packs.required": "required entrypoint",
  "packs.open": "Open",
  "packs.routed": "In active Profile",
  "packs.available": "Available",
  "packs.fileCount": "{count} Markdown file(s)",
  "packs.addFile": "+ Markdown",
  "packs.addFileTitle": "Append instruction source",
  "packs.addFilePath": "Relative Markdown path",
  "packs.addFilePreview": "Preview file",
  "packs.addFileConfirm": "Add Markdown",
  "packs.create.eyebrow": "New manifest",
  "packs.create.title": "Create Rule Pack",
  "packs.create.body":
    "Creation is additive. A preview is required and an existing directory is never overwritten.",
  "packs.create.id": "Stable id",
  "packs.create.idHint": "Lowercase letters, digits, hyphen, or underscore",
  "packs.create.name": "Display name",
  "packs.create.namePlaceholder": "Team rules",
  "packs.create.description": "Description",
  "packs.create.entrypoint": "Required and always rendered first",
  "packs.create.preview": "Preview Rule Pack",
  "packs.create.confirm": "Create Rule Pack",
  "profiles.eyebrow": "Machine routing sheets",
  "profiles.title": "Profiles",
  "profiles.intro":
    "Profiles are syncable ordered Pack lists. Which Profile is active remains a local choice on every machine.",
  "profiles.machineLabel": "Selection scope",
  "profiles.machineLocal": "THIS MACHINE ONLY",
  "profiles.syncNote": "Profile definitions sync; active selection, runtime, and current do not.",
  "profiles.rollback": "Preview latest library rollback",
  "profiles.rollbackPreview": "Restore drift-safe snapshot {id}",
  "profiles.rollbackConfirm": "Confirm rollback",
  "profiles.listLabel": "Profile routing sheets",
  "profiles.noDescription": "No description",
  "profiles.activeHere": "ACTIVE HERE",
  "profiles.packOrder": "Ordered Rule Pack route",
  "profiles.notSelected": "Available on this machine",
  "profiles.previewSwitch": "Preview switch",
  "profiles.refresh": "Preview refresh",
  "profiles.confirmSwitch": "Switch Profile",
  "profiles.confirmRefresh": "Refresh runtime",
  "profiles.create.eyebrow": "New routing sheet",
  "profiles.create.title": "Create Profile",
  "profiles.create.body": "Choose Packs and arrange their render order before writing the Profile file.",
  "profiles.create.id": "Stable id",
  "profiles.create.name": "Display name",
  "profiles.create.namePlaceholder": "Client work",
  "profiles.create.description": "Description",
  "profiles.create.selectPacks": "Available Rule Packs",
  "profiles.create.order": "Render order",
  "profiles.create.orderEmpty": "Select at least one Pack.",
  "profiles.create.moveUp": "Move Pack earlier",
  "profiles.create.moveDown": "Move Pack later",
  "profiles.create.preview": "Preview Profile",
  "profiles.create.confirm": "Create Profile",
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
  "brand.subtitle": "本地规则路由",
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
  "readout.aligned": "稳定 current 入口",
  "notice.dismiss": "关闭通知",
  "source.missing.eyebrow": "尚无规范源",
  "source.missing.title": "从一份中立规则开始。",
  "source.missing.body":
    "Agent Rules Manager 将创建 {path}。检查并确认接入之前，不会修改任何 Agent 原生文件。",
  "source.missing.initialize": "初始化规则库",
  "notice.sourceOpened": "已使用 {app} 打开 {file}。",
  "notice.sourceRefreshed": "已重新检查规则包源、runtime 和 Agent 原生路径。",
  "notice.demoOpen": "浏览器演示模式无法打开本地应用。",
  "notice.applied": "已应用接入选择：{agents}。",
  "notice.aligned": "所选接入状态已生效。",
  "notice.noConnectionChanges": "请先选择至少一个 Agent 接入中央规则或改为独立使用。",
  "notice.restored": "已恢复快照 {id}。",
  "notice.initialized": "已创建规则包库，并在本机选中默认 Profile。",
  "notice.packCreated": "已创建规则包 {name}。",
  "notice.packFileAdded": "已新增 {path}；如当前 Profile 使用此规则包，请刷新 runtime。",
  "notice.profileCreated": "已创建 Profile {name}。",
  "notice.profileActivated": "本机已切换到 Profile {id}。",
  "notice.libraryRestored": "已恢复规则库快照 {id}。",
  "projection.eyebrow": "原生投射",
  "projection.title": "一个稳定入口，路由到所有 Agent",
  "projection.hint": "Profile 可在 current/AGENTS.md 后切换，无需重新接入 Agent。",
  "projection.canonical": "当前 Profile",
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
  "projection.detail.legacyLink": "仍指向迁移前的根目录规则路径。",
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
  "projection.syncActiveTitle": "Agent 原生接入已对齐",
  "projection.syncActiveBody": "已接入 Agent 通过一个稳定路径读取本机当前 Profile。",
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
  "projection.step.createLink": "将创建指向 current/AGENTS.md 的符号链接。",
  "projection.step.createIndependentFile": "将把托管软链接替换为当前规则的独立副本。",
  "projection.step.detachManagedInclude": "将把旧版托管引用替换为独立规则内容。",
  "projection.step.migrateLegacyInclude": "将把旧版托管引用迁移为符号链接。",
  "projection.step.refreshManagedBlock": "将刷新现有托管引用。",
  "projection.step.replaceLegacyLink": "将旧版托管链接迁移到 current/AGENTS.md。",
  "projection.step.change": "将安全建立接入关系。",
  "source.file.eyebrow": "当前规则包源文件",
  "source.file.title": "使用你熟悉的工具编辑源规则",
  "source.file.path": "可编辑源文件",
  "source.file.description": "保存后返回此处，刷新当前 Profile 的 runtime。",
  "source.file.digest": "内容摘要",
  "source.file.modified": "最后修改",
  "source.file.unknownTime": "暂无记录",
  "source.file.externalTitle": "外部编辑边界",
  "source.file.externalBody": "规则包文件由用户维护并可同步；生成 runtime 与回滚都不会替换它。",
  "source.file.watch": "窗口重新获得焦点时检查源文件摘要",
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
  "libraryPlan.ready": "文件变更预览已就绪",
  "libraryPlan.blocked": "文件变更已阻止",
  "libraryPlan.changeCount": "{count} 项文件系统变更",
  "libraryPlan.cancel": "取消",
  "libraryPlan.applying": "正在应用…",
  "libraryPlan.previewing": "正在生成预览…",
  "setup.emptyEyebrow": "空规则库",
  "setup.emptyTitle": "建立第一条规则包路由。",
  "setup.legacyEyebrow": "发现旧版规则源",
  "setup.legacyTitle": "先创建回滚快照，再迁移 AGENTS.md。",
  "setup.conflictEyebrow": "规则库冲突",
  "setup.conflictTitle": "无法安全初始化这个目录。",
  "setup.preview": "预览初始化",
  "setup.confirmCreate": "创建规则库",
  "setup.confirmImport": "迁移旧规则",
  "runtimeGate.eyebrow": "本机 runtime",
  "runtimeGate.selectTitle": "为这台机器选择一个 Profile。",
  "runtimeGate.refreshTitle": "当前 Profile 需要重新生成 runtime。",
  "runtimeGate.openProfiles": "打开 Profile",
  "runtime.current": "runtime 已是最新",
  "runtime.missing": "尚无 runtime",
  "runtime.stale": "源规则已变化",
  "runtime.conflict": "runtime 有冲突",
  "packs.eyebrow": "可版本化的指令清单",
  "packs.title": "规则包",
  "packs.intro": "每个规则包都有必需的 AGENTS.md，还可按顺序附加 Markdown。规则包可以共存，由 Profile 决定组合方式。",
  "packs.sync.title": "多机器同步边界",
  "packs.legacy.title": "发现意外的根目录规则文件",
  "packs.legacy.body": "当前规则库只通过 current/AGENTS.md 路由规则包。根目录 AGENTS.md 不在生效链路中，程序也不会自动覆盖它。",
  "packs.listLabel": "规则包清单",
  "packs.ledger.pack": "规则包清单",
  "packs.ledger.sources": "有序指令源",
  "packs.ledger.state": "当前路由",
  "packs.noDescription": "暂无说明",
  "packs.required": "必需入口",
  "packs.open": "打开",
  "packs.routed": "位于当前 Profile",
  "packs.available": "可用",
  "packs.fileCount": "{count} 个 Markdown 文件",
  "packs.addFile": "+ Markdown",
  "packs.addFileTitle": "追加指令源",
  "packs.addFilePath": "Markdown 相对路径",
  "packs.addFilePreview": "预览文件变更",
  "packs.addFileConfirm": "添加 Markdown",
  "packs.create.eyebrow": "新清单",
  "packs.create.title": "创建规则包",
  "packs.create.body": "创建只会新增文件。必须先预览，已有目录绝不会被覆盖。",
  "packs.create.id": "稳定 ID",
  "packs.create.idHint": "仅小写字母、数字、连字符或下划线",
  "packs.create.name": "显示名称",
  "packs.create.namePlaceholder": "团队规则",
  "packs.create.description": "说明",
  "packs.create.entrypoint": "必需，且始终最先渲染",
  "packs.create.preview": "预览规则包",
  "packs.create.confirm": "创建规则包",
  "profiles.eyebrow": "本机规则接线单",
  "profiles.title": "Profile",
  "profiles.intro": "Profile 是可同步的有序规则包列表；每台机器各自决定启用哪个 Profile。",
  "profiles.machineLabel": "选择范围",
  "profiles.machineLocal": "仅当前机器",
  "profiles.syncNote": "Profile 定义参与同步；启用状态、runtime 和 current 不同步。",
  "profiles.rollback": "预览最近一次规则库回滚",
  "profiles.rollbackPreview": "恢复带漂移保护的快照 {id}",
  "profiles.rollbackConfirm": "确认回滚",
  "profiles.listLabel": "Profile 接线单",
  "profiles.noDescription": "暂无说明",
  "profiles.activeHere": "本机启用",
  "profiles.packOrder": "规则包渲染顺序",
  "profiles.notSelected": "本机可用",
  "profiles.previewSwitch": "预览切换",
  "profiles.refresh": "预览刷新",
  "profiles.confirmSwitch": "切换 Profile",
  "profiles.confirmRefresh": "刷新 runtime",
  "profiles.create.eyebrow": "新接线单",
  "profiles.create.title": "创建 Profile",
  "profiles.create.body": "选择规则包并安排渲染顺序，再写入 Profile 文件。",
  "profiles.create.id": "稳定 ID",
  "profiles.create.name": "显示名称",
  "profiles.create.namePlaceholder": "客户项目",
  "profiles.create.description": "说明",
  "profiles.create.selectPacks": "可用规则包",
  "profiles.create.order": "渲染顺序",
  "profiles.create.orderEmpty": "至少选择一个规则包。",
  "profiles.create.moveUp": "将规则包前移",
  "profiles.create.moveDown": "将规则包后移",
  "profiles.create.preview": "预览 Profile",
  "profiles.create.confirm": "创建 Profile",
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
