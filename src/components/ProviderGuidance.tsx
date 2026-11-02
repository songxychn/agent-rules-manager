import { useI18n } from "../lib/i18n";

export function ProviderGuidance() {
  const { locale } = useI18n();
  const zh = locale === "zh-CN";
  return (
    <section className="provider-guidance-bar">
      <details className="provider-guidance">
        <summary>{zh ? "使用 Ollama、Z.ai / GLM 或其他模型服务" : "Using Ollama, Z.ai / GLM, or another model provider"}</summary>
        <p>{zh ? "选择实际使用的客户端接入：Ollama 可配合 Claude Code、Codex、OpenCode；Z.ai / GLM 可配合 Claude Code、Cline、OpenCode。规则由客户端读取，切换模型不需要重复投射，也不会改动 API Key 或模型配置。" : "Connect the client you use: Ollama works with Claude Code, Codex, and OpenCode; Z.ai / GLM works with Claude Code, Cline, and OpenCode. The client loads rules, so switching models requires no duplicate projection or changes to API keys and model settings."}</p>
        <a href="https://docs.ollama.com/quickstart" target="_blank" rel="noreferrer">Ollama</a>{" · "}<a href="https://docs.z.ai/devpack/overview" target="_blank" rel="noreferrer">Z.ai / GLM</a>
      </details>
    </section>
  );
}
