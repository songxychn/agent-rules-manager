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
  "notice.sourceRefreshed": "Rechecked Profile sources, runtime, and native paths.",
  "notice.demoOpen": "The browser demo cannot open local applications.",
  "notice.applied": "Applied connection choices: {agents}.",
  "notice.appliedWithBackup":
    "Applied connection choices: {agents}. Preserved the previous files in {path}.",
  "notice.aligned": "The selected connection choices are already active.",
  "notice.noConnectionChanges": "Choose at least one agent to connect or make independent.",
  "notice.restored": "Restored snapshot {id}.",
  "notice.initialized": "Created the Profile library and selected its default Profile.",
  "notice.profileCreated": "Created Profile {name}.",
  "notice.profileFileAdded": "Added {path}; refresh the Profile to update its runtime.",
  "notice.profileFileRemoved":
    "Deleted Markdown source {path}. A drift-safe rollback snapshot is available.",
  "notice.profileDeleted": "Deleted Profile {id}. A drift-safe rollback snapshot is available.",
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
  "projection.selectionPendingTitle": "{count} connection choices changed",
  "projection.selectionPendingBody":
    "Continue to confirm the native Agent paths that will change.",
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
    "Regular files can be preserved and replaced after review; unexpected links remain blocked.",
  "projection.blockedTitle": "Connection blocked",
  "projection.blockedBody": "An unexpected symbolic link is present. Nothing was overwritten.",
  "projection.backupConfirmationTitle": "{count} existing files need explicit confirmation",
  "projection.backupConfirmationBody":
    "Confirm to preserve readable copies in application backups before replacing the native files.",
  "projection.connectionChangesReady": "Confirm {count} connection changes",
  "projection.connectionChangesBody":
    "The highlighted agents will change after confirmation. A rollback snapshot is created first.",
  "projection.inspect": "Continue",
  "projection.inspecting": "Preparing confirmation…",
  "projection.recheck": "Refresh confirmation",
  "projection.applying": "Saving…",
  "projection.applyOne": "Confirm change",
  "projection.applyMany": "Confirm {count} changes",
  "projection.backupAndApplyOne": "Back up file and apply",
  "projection.backupAndApplyMany": "Back up files and apply {count} changes",
  "projection.connectOne": "Connect 1 target",
  "projection.connectMany": "Connect {count} targets",
  "projection.step.conflict": "An unexpected symbolic link was detected.",
  "projection.step.backupAndCreateLink":
    "Will preserve the complete existing file, then replace it with a symbolic link.",
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
  "source.file.eyebrow": "Active Profile source",
  "source.file.title": "Edit source rules in your own tools",
  "source.file.path": "editable source",
  "source.file.description":
    "After saving, return here and refresh the active Profile runtime.",
  "source.file.digest": "Content digest",
  "source.file.modified": "Last modified",
  "source.file.unknownTime": "Not available",
  "source.file.externalTitle": "External editing boundary",
  "source.file.externalBody":
    "Profile files are user-owned and syncable. Generated runtime and rollback never replace them.",
  "source.file.watch": "Rechecks source digests when this window regains focus",
  "source.file.refresh": "Recheck now",
  "source.file.refreshing": "Rechecking…",
  "source.open.action": "Open with",
  "source.open.with": "Open {file} with {app}",
  "source.open.default": "Default app",
  "source.open.unavailable": "No application available",
  "source.open.opening": "Opening…",
  "source.open.menu": "Choose how to open {file}",
  "source.open.editors": "Editors",
  "source.open.system": "System locations",
  "libraryPlan.confirmStamp": "CONFIRM",
  "libraryPlan.blocked": "Mutation blocked",
  "libraryPlan.details": "Show {count} affected path(s)",
  "libraryPlan.noChanges": "No files will be changed.",
  "libraryPlan.snapshotNote": "A rollback snapshot will be created before changes.",
  "libraryPlan.cancel": "Cancel",
  "libraryPlan.close": "Close",
  "libraryPlan.applying": "Saving…",
  "libraryPlan.preparing": "Preparing confirmation…",
  "setup.emptyEyebrow": "Empty library",
  "setup.emptyTitle": "Build the first Profile route.",
  "setup.legacyEyebrow": "Legacy source found",
  "setup.legacyTitle": "Migrate AGENTS.md with a rollback snapshot.",
  "setup.upgradeEyebrow": "Schema upgrade available",
  "setup.upgradeTitle": "Move Rule Pack content into Profiles safely.",
  "setup.conflictEyebrow": "Library conflict",
  "setup.conflictTitle": "This directory cannot be initialized safely.",
  "setup.startCreate": "Initialize library",
  "setup.startImport": "Migrate legacy rules",
  "setup.startUpgrade": "Upgrade library",
  "setup.createQuestion": "Create this rule library?",
  "setup.createPrompt":
    "This creates the default Profile, its AGENTS.md, and this machine's runtime.",
  "setup.importQuestion": "Migrate the existing AGENTS.md?",
  "setup.importPrompt":
    "This moves the existing rules into a default Profile and keeps a rollback snapshot.",
  "setup.upgradeQuestion": "Upgrade this rule library?",
  "setup.upgradePrompt":
    "This moves legacy Rule Pack content into Profiles and keeps a rollback snapshot.",
  "setup.confirmCreate": "Confirm creation",
  "setup.confirmImport": "Confirm migration",
  "setup.confirmUpgrade": "Confirm upgrade",
  "runtimeGate.eyebrow": "Machine runtime",
  "runtimeGate.selectTitle": "Choose a Profile for this machine.",
  "runtimeGate.refreshTitle": "The active Profile needs a fresh runtime.",
  "runtimeGate.openProfiles": "Open Profiles",
  "runtime.current": "Runtime current",
  "runtime.missing": "Runtime missing",
  "runtime.stale": "Sources changed",
  "runtime.conflict": "Runtime conflict",
  "profiles.eyebrow": "Versioned rule routes",
  "profiles.title": "Profiles",
  "profiles.intro":
    "Each Profile owns its ordered Markdown sources. The active Profile remains a local choice on every machine.",
  "profiles.machineLabel": "Selection scope",
  "profiles.machineLocal": "THIS MACHINE ONLY",
  "profiles.syncNote": "schema.json and profiles/** sync; active selection, runtime, and current do not.",
  "profiles.rollback": "Restore latest change",
  "profiles.rollbackQuestion": "Restore the latest change?",
  "profiles.rollbackPrompt": "This restores snapshot {id} and undoes the latest library change.",
  "profiles.rollbackNote": "Restore stops if an affected path changed after the snapshot.",
  "profiles.rollbackConfirm": "Confirm restore",
  "profiles.listLabel": "Profile manifests and sources",
  "profiles.noDescription": "No description",
  "profiles.activeHere": "ACTIVE HERE",
  "profiles.files": "Ordered Profile sources",
  "profiles.required": "required",
  "profiles.open": "Open",
  "profiles.fileCount": "{count} Markdown file(s)",
  "profiles.output": "Rendered output",
  "profiles.addFile": "+ Markdown source",
  "profiles.addFileTitle": "Append Profile source",
  "profiles.addFilePath": "Relative Markdown path",
  "profiles.addFileSubmit": "Add Markdown",
  "profiles.addFileQuestion": "Add this Markdown source?",
  "profiles.addFilePrompt": "This adds {path} to {profile} and creates its source file.",
  "profiles.addFileConfirm": "Confirm addition",
  "profiles.removeFile": "Delete",
  "profiles.removeFileLabel": "Delete Markdown source {path}",
  "profiles.removeFileQuestion": "Delete this Markdown source?",
  "profiles.removeFilePrompt":
    "This removes {path} from {profile}. The active runtime will need a refresh.",
  "profiles.removeFileConfirm": "Confirm deletion",
  "profiles.delete": "Delete Profile",
  "profiles.deleteActiveTitle": "Switch to another Profile before deleting this active Profile",
  "profiles.deleteQuestion": "Delete Profile {profile}?",
  "profiles.deletePrompt":
    "This deletes {count} Markdown source(s) owned by {profile}. The latest snapshot can restore them.",
  "profiles.deleteConfirm": "Confirm deletion",
  "profiles.notSelected": "Available on this machine",
  "profiles.switch": "Switch Profile",
  "profiles.refresh": "Refresh runtime",
  "profiles.switchQuestion": "Switch to this Profile?",
  "profiles.switchPrompt":
    "This switches only this machine to {profile} and rebuilds its runtime.",
  "profiles.refreshQuestion": "Refresh this runtime?",
  "profiles.refreshPrompt": "This rebuilds the local runtime from {profile}'s current sources.",
  "profiles.confirmSwitch": "Confirm switch",
  "profiles.confirmRefresh": "Confirm refresh",
  "profiles.create.eyebrow": "New Profile",
  "profiles.create.title": "Create Profile",
  "profiles.create.body":
    "Creation is additive. Every Profile starts with its own required AGENTS.md source.",
  "profiles.create.id": "Stable id",
  "profiles.create.name": "Display name",
  "profiles.create.namePlaceholder": "Client work",
  "profiles.create.description": "Description",
  "profiles.create.entrypoint": "Created with the Profile and rendered first",
  "profiles.create.submit": "Create Profile",
  "profiles.create.question": "Create this Profile?",
  "profiles.create.prompt":
    "This creates {name} ({id}) with its required AGENTS.md and leaves existing Profiles unchanged.",
  "profiles.create.confirm": "Confirm creation",
  "profiles.create.close": "Close create Profile dialog",
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
  "nav.profiles": "Profile",
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
  "notice.sourceRefreshed": "已重新检查 Profile 源、runtime 和 Agent 原生路径。",
  "notice.demoOpen": "浏览器演示模式无法打开本地应用。",
  "notice.applied": "已应用接入选择：{agents}。",
  "notice.appliedWithBackup": "已应用接入选择：{agents}。原文件已备份到 {path}。",
  "notice.aligned": "所选接入状态已生效。",
  "notice.noConnectionChanges": "请先选择至少一个 Agent 接入中央规则或改为独立使用。",
  "notice.restored": "已恢复快照 {id}。",
  "notice.initialized": "已创建 Profile 规则库，并在本机选中默认 Profile。",
  "notice.profileCreated": "已创建 Profile {name}。",
  "notice.profileFileAdded": "已新增 {path}；刷新 Profile 后会更新 runtime。",
  "notice.profileFileRemoved": "已删除 Markdown 源 {path}，并保留了带漂移保护的回滚快照。",
  "notice.profileDeleted": "已删除 Profile {id}，并保留了带漂移保护的回滚快照。",
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
  "projection.selectionPendingTitle": "已修改 {count} 项接入选择",
  "projection.selectionPendingBody": "继续后确认将要修改的 Agent 原生路径。",
  "projection.noneDetectedTitle": "未检测到受支持的 Agent",
  "projection.noneDetectedBody": "安装或启动受支持的 Agent 后，再重新检查当前工作区。",
  "projection.syncActiveTitle": "Agent 原生接入已对齐",
  "projection.syncActiveBody": "已接入 Agent 通过一个稳定路径读取本机当前 Profile。",
  "projection.attentionTitle": "{count} 个 Agent 尚未完成接入",
  "projection.attentionBody": "先检查将要建立的引用关系，确认后再写入 Agent 原生路径。",
  "projection.conflictDetectedTitle": "检测到 {count} 个接入冲突",
  "projection.conflictDetectedBody": "常规文件可在检查后备份接管；指向未知位置的软链接仍会阻止操作。",
  "projection.blockedTitle": "接入已阻止",
  "projection.blockedBody": "目标存在指向未知位置的软链接；未执行任何写入。",
  "projection.backupConfirmationTitle": "有 {count} 个原文件需要明确确认",
  "projection.backupConfirmationBody":
    "确认后会先在应用备份目录保存可直接打开的完整副本，再替换 Agent 原生文件。",
  "projection.connectionChangesReady": "确认 {count} 项接入修改",
  "projection.connectionChangesBody": "确认后会修改高亮的 Agent，并先创建回滚快照。",
  "projection.inspect": "继续",
  "projection.inspecting": "正在准备确认信息…",
  "projection.recheck": "刷新确认信息",
  "projection.applying": "正在保存…",
  "projection.applyOne": "确认修改",
  "projection.applyMany": "确认 {count} 项修改",
  "projection.backupAndApplyOne": "备份原文件并应用",
  "projection.backupAndApplyMany": "备份原文件并应用 {count} 项调整",
  "projection.connectOne": "确认接入 1 项",
  "projection.connectMany": "确认接入 {count} 项",
  "projection.step.conflict": "检测到指向未知位置的软链接。",
  "projection.step.backupAndCreateLink": "将完整备份现有文件，再替换为指向中央规则的软链接。",
  "projection.step.addInclude": "将保留 Agent 专属规则并添加托管引用。",
  "projection.step.createLink": "将创建指向 current/AGENTS.md 的符号链接。",
  "projection.step.createIndependentFile": "将把托管软链接替换为当前规则的独立副本。",
  "projection.step.detachManagedInclude": "将把旧版托管引用替换为独立规则内容。",
  "projection.step.migrateLegacyInclude": "将把旧版托管引用迁移为符号链接。",
  "projection.step.refreshManagedBlock": "将刷新现有托管引用。",
  "projection.step.replaceLegacyLink": "将旧版托管链接迁移到 current/AGENTS.md。",
  "projection.step.change": "将安全建立接入关系。",
  "source.file.eyebrow": "当前 Profile 源文件",
  "source.file.title": "使用你熟悉的工具编辑源规则",
  "source.file.path": "可编辑源文件",
  "source.file.description": "保存后返回此处，刷新当前 Profile 的 runtime。",
  "source.file.digest": "内容摘要",
  "source.file.modified": "最后修改",
  "source.file.unknownTime": "暂无记录",
  "source.file.externalTitle": "外部编辑边界",
  "source.file.externalBody": "Profile 文件由用户维护并可同步；生成 runtime 与回滚都不会替换它。",
  "source.file.watch": "窗口重新获得焦点时检查源文件摘要",
  "source.file.refresh": "立即检查",
  "source.file.refreshing": "正在检查…",
  "source.open.action": "打开方式",
  "source.open.with": "使用 {app} 打开 {file}",
  "source.open.default": "系统默认应用",
  "source.open.unavailable": "没有可用应用",
  "source.open.opening": "正在打开…",
  "source.open.menu": "选择 {file} 的打开方式",
  "source.open.editors": "编辑器",
  "source.open.system": "系统位置",
  "libraryPlan.confirmStamp": "确认",
  "libraryPlan.blocked": "文件变更已阻止",
  "libraryPlan.details": "查看 {count} 个涉及路径",
  "libraryPlan.noChanges": "不会修改任何文件。",
  "libraryPlan.snapshotNote": "修改前会先创建可回滚快照。",
  "libraryPlan.cancel": "取消",
  "libraryPlan.close": "关闭",
  "libraryPlan.applying": "正在保存…",
  "libraryPlan.preparing": "正在准备确认信息…",
  "setup.emptyEyebrow": "空规则库",
  "setup.emptyTitle": "建立第一个 Profile 路由。",
  "setup.legacyEyebrow": "发现旧版规则源",
  "setup.legacyTitle": "先创建回滚快照，再迁移 AGENTS.md。",
  "setup.upgradeEyebrow": "可升级规则库结构",
  "setup.upgradeTitle": "安全地把旧规则包内容收进 Profile。",
  "setup.conflictEyebrow": "规则库冲突",
  "setup.conflictTitle": "无法安全初始化这个目录。",
  "setup.startCreate": "初始化规则库",
  "setup.startImport": "迁移旧规则",
  "setup.startUpgrade": "升级规则库",
  "setup.createQuestion": "创建这个规则库？",
  "setup.createPrompt": "将创建默认 Profile、必需的 AGENTS.md 和本机 runtime。",
  "setup.importQuestion": "迁移现有 AGENTS.md？",
  "setup.importPrompt": "将现有规则移入默认 Profile，并保留可回滚快照。",
  "setup.upgradeQuestion": "升级这个规则库？",
  "setup.upgradePrompt": "将旧规则包内容迁移到 Profile 结构，并保留可回滚快照。",
  "setup.confirmCreate": "确认创建",
  "setup.confirmImport": "确认迁移",
  "setup.confirmUpgrade": "确认升级",
  "runtimeGate.eyebrow": "本机 runtime",
  "runtimeGate.selectTitle": "为这台机器选择一个 Profile。",
  "runtimeGate.refreshTitle": "当前 Profile 需要重新生成 runtime。",
  "runtimeGate.openProfiles": "打开 Profile",
  "runtime.current": "runtime 已是最新",
  "runtime.missing": "尚无 runtime",
  "runtime.stale": "源规则已变化",
  "runtime.conflict": "runtime 有冲突",
  "profiles.eyebrow": "可版本化的规则路由",
  "profiles.title": "Profile",
  "profiles.intro": "每个 Profile 直接持有自己的有序 Markdown 源；每台机器各自决定启用哪个 Profile。",
  "profiles.machineLabel": "选择范围",
  "profiles.machineLocal": "仅当前机器",
  "profiles.syncNote": "schema.json 与 profiles/** 参与同步；启用状态、runtime 和 current 不同步。",
  "profiles.rollback": "恢复最近一次修改",
  "profiles.rollbackQuestion": "恢复最近一次修改？",
  "profiles.rollbackPrompt": "将恢复快照 {id}，撤销最近一次规则库修改。",
  "profiles.rollbackNote": "若相关路径在快照后发生变化，将停止恢复。",
  "profiles.rollbackConfirm": "确认恢复",
  "profiles.listLabel": "Profile 清单与源文件",
  "profiles.noDescription": "暂无说明",
  "profiles.activeHere": "本机启用",
  "profiles.files": "Profile 有序源文件",
  "profiles.required": "必需",
  "profiles.open": "打开",
  "profiles.fileCount": "{count} 个 Markdown 文件",
  "profiles.output": "渲染输出",
  "profiles.addFile": "+ Markdown 源",
  "profiles.addFileTitle": "追加 Profile 源文件",
  "profiles.addFilePath": "Markdown 相对路径",
  "profiles.addFileSubmit": "添加 Markdown",
  "profiles.addFileQuestion": "添加这个 Markdown 源？",
  "profiles.addFilePrompt": "将把 {path} 加入 {profile}，并创建对应源文件。",
  "profiles.addFileConfirm": "确认添加",
  "profiles.removeFile": "删除",
  "profiles.removeFileLabel": "删除 Markdown 源 {path}",
  "profiles.removeFileQuestion": "删除这个 Markdown 源？",
  "profiles.removeFilePrompt": "将从 {profile} 移除 {path}，当前 runtime 随后需要刷新。",
  "profiles.removeFileConfirm": "确认删除",
  "profiles.delete": "删除 Profile",
  "profiles.deleteActiveTitle": "请先切换到其他 Profile，再删除当前启用的 Profile",
  "profiles.deleteQuestion": "删除 Profile {profile}？",
  "profiles.deletePrompt": "将删除 {profile} 持有的 {count} 个 Markdown 源，可用最近快照恢复。",
  "profiles.deleteConfirm": "确认删除",
  "profiles.notSelected": "本机可用",
  "profiles.switch": "切换 Profile",
  "profiles.refresh": "刷新 runtime",
  "profiles.switchQuestion": "切换到这个 Profile？",
  "profiles.switchPrompt": "只会把当前机器切换到 {profile}，并重新生成 runtime。",
  "profiles.refreshQuestion": "刷新当前 runtime？",
  "profiles.refreshPrompt": "将根据 {profile} 的现有源文件重新生成本机 runtime。",
  "profiles.confirmSwitch": "确认切换",
  "profiles.confirmRefresh": "确认刷新",
  "profiles.create.eyebrow": "新 Profile",
  "profiles.create.title": "创建 Profile",
  "profiles.create.body": "创建只会新增文件。每个 Profile 都从自己的必需 AGENTS.md 源开始。",
  "profiles.create.id": "稳定 ID",
  "profiles.create.name": "显示名称",
  "profiles.create.namePlaceholder": "客户项目",
  "profiles.create.description": "说明",
  "profiles.create.entrypoint": "随 Profile 创建，并始终最先渲染",
  "profiles.create.submit": "创建 Profile",
  "profiles.create.question": "创建这个 Profile？",
  "profiles.create.prompt": "将创建 {name}（{id}）及其必需的 AGENTS.md，不修改现有 Profile。",
  "profiles.create.confirm": "确认创建",
  "profiles.create.close": "关闭创建 Profile 对话框",
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
