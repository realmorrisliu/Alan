## 验证闭环

每次修改 `crates/` 下的 Rust 代码后，运行 `just verify` 确认核心流程正常。
如果本机配置了 `~/.alan` LLM，可以运行 `just verify-full` 做端到端验证。
验证失败时，阅读输出日志定位问题并修复。
