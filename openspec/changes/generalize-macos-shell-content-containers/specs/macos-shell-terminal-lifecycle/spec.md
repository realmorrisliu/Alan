## ADDED Requirements

### Requirement: Terminal lifecycle is scoped to terminal content
macOS shell 的 terminal runtime lifecycle SHALL 只适用于 `terminal` ContentInstance。非 terminal
ContentInstance MUST NOT 分配 terminal runtime、shell process、Ghostty surface 或 terminal delivery queue。

#### Scenario: Settings pane becomes visible
- **WHEN** 用户选择承载 settings content 的 pane
- **THEN** alan 渲染 settings surface
- **AND** terminal runtime registry 不为该 PaneSlot 或 ContentInstance 创建 shell process 或 Ghostty host

#### Scenario: Markdown pane receives terminal text command
- **WHEN** terminal text command 的目标 PaneSlot 承载 markdown content
- **THEN** terminal lifecycle 不接收该 delivery
- **AND** control response 报告 stable unsupported-content error

#### Scenario: Live terminal pane remains continuous after model migration
- **WHEN** 当前进程内仍属于 shell state 的 terminal pane 从旧 terminal-only model 迁移到 content-container model
- **THEN** 该 terminal ContentInstance 的 process、scrollback、metadata、pending delivery 和 reattachment 语义保持连续
- **AND** runtime continuity 绑定到 `content_id`

#### Scenario: Terminal content restored from workspace manifest
- **WHEN** alan after app restart 从 workspace manifest materialize 出 terminal ContentInstance
- **THEN** terminal lifecycle 创建新的 terminal runtime 和 renderer surface
- **AND** alan MUST NOT 声称恢复上一轮 app 进程中的 terminal process、scrollback 或 delivery queue
- **AND** runtime continuity 从本轮 materialization 之后开始绑定到该 `content_id`

### Requirement: Terminal adapter owns terminal-specific projection
Terminal content adapter SHALL 负责将 terminal runtime metadata 投影为 shell-visible title、cwd、
attention、surface readiness、alan binding 和 terminal command capabilities，并以 `content_id`
作为 runtime identity。

#### Scenario: Background terminal metadata updates
- **WHEN** 后台 terminal content 报告 cwd、title、attention 或 process status 变化
- **THEN** terminal adapter 更新该 content/pane 的 shell projection
- **AND** 用户当前聚焦的非 terminal pane 不被抢占

#### Scenario: Terminal content closes
- **WHEN** 用户关闭承载 terminal content 的 pane
- **THEN** terminal adapter 以 `content_id` 调用 terminal runtime finalizer
- **AND** shell layout 删除该 PaneSlot 后不保留可投递 terminal target

#### Scenario: Terminal content moves between pane slots
- **WHEN** terminal ContentInstance 从一个 PaneSlot 移动或重挂到另一个 PaneSlot
- **THEN** terminal runtime handle、scrollback、pending delivery 和 metadata 保持绑定到同一个 `content_id`
- **AND** terminal host focus 解析到新的 PaneSlot 位置
