import { useState } from "react";
import { useI18n } from "../lib/i18n";

export function AgentScopePanel({ projectRoot, disabled, onChange }: {
  projectRoot: string;
  disabled: boolean;
  onChange: (root: string) => Promise<void>;
}) {
  const { locale } = useI18n();
  const zh = locale === "zh-CN";
  const [input, setInput] = useState(projectRoot);
  const [editing, setEditing] = useState(false);
  return (
    <section className="agent-scope-panel" aria-label={zh ? "规则适用范围" : "Rule scope"}>
      <div className="agent-scope-heading">
        <strong>{projectRoot ? (zh ? "项目规则" : "Project rules") : (zh ? "全局规则" : "Global rules")}</strong>
        {projectRoot && <code title={projectRoot}>{projectRoot}</code>}
        <button type="button" className="button button-ghost button-small" aria-expanded={editing} onClick={() => setEditing(!editing)} disabled={disabled}>{zh ? "切换范围" : "Change scope"}</button>
      </div>
      {editing && <form onSubmit={(event) => { event.preventDefault(); void onChange(input.trim()); }}>
        <label htmlFor="agent-project-root">{zh ? "项目目录" : "Project directory"}</label>
        <input id="agent-project-root" value={input} onChange={(event) => setInput(event.target.value)} placeholder={zh ? "/绝对路径/项目目录" : "/absolute/path/to/project"} disabled={disabled} />
        <button className="button button-secondary button-small" disabled={disabled || !input.trim()}>{zh ? "查看项目" : "View project"}</button>
        {projectRoot && <button type="button" className="button button-ghost button-small" disabled={disabled} onClick={() => { setInput(""); void onChange(""); }}>{zh ? "返回全局" : "Global rules"}</button>}
      </form>}
      {editing && <small>{zh ? "Cursor、Copilot IDE、Continue、Junie 使用项目规则。" : "Cursor, Copilot IDE, Continue, and Junie use project rules."}</small>}
      {editing && projectRoot && <small>{zh ? "接入仅作用于此项目；规则跟随当前机器启用的 Profile。项目软链接依赖本机规则库，不适合直接提交给其他机器使用。" : "Applies to this project and follows this machine’s active Profile. Links depend on the local library; do not commit them as portable team rules."}</small>}
      <details className="provider-guidance">
        <summary>{zh ? "使用 Ollama、Z.ai / GLM 或其他模型服务" : "Using Ollama, Z.ai / GLM, or another model provider"}</summary>
        <p>{zh ? "选择实际使用的客户端接入：Ollama 可配合 Claude Code、Codex、OpenCode；Z.ai / GLM 可配合 Claude Code、Cline、OpenCode。规则由客户端读取，切换模型不需要重复投射，也不会改动 API Key 或模型配置。" : "Connect the client you use: Ollama works with Claude Code, Codex, and OpenCode; Z.ai / GLM works with Claude Code, Cline, and OpenCode. The client loads rules, so switching models requires no duplicate projection or changes to API keys and model settings."}</p>
        <a href="https://docs.ollama.com/quickstart" target="_blank" rel="noreferrer">Ollama</a>{" · "}<a href="https://docs.z.ai/devpack/overview" target="_blank" rel="noreferrer">Z.ai / GLM</a>
      </details>
    </section>
  );
}
