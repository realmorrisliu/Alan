## ADDED Requirements

### Requirement: Content container model has focused tests
Apple client SHALL 为 v0.2 content-container model、旧状态迁移、mixed split、content-aware
command validation 和 content-keyed terminal runtime continuity 提供 focused 自动化测试或明确的人工验证记录。

#### Scenario: Old terminal state migrates
- **WHEN** 测试加载 `persist-macos-shell-workspaces` 产生的 terminal-only workspace manifest
- **THEN** shell model 迁移出 PaneSlot 和 terminal ContentInstance
- **AND** focused space、focused tab、focused PaneSlot、pin/live snapshot 和 terminal metadata projection 保持一致

#### Scenario: Legacy shell state is not restored as workspace authority
- **WHEN** 测试环境同时存在旧 `shell-state-window_main.json` 和 workspace manifest
- **THEN** app restore 使用 workspace manifest materializer
- **AND** 旧 shell-state 只作为 runtime/control-plane projection 或诊断输入存在

#### Scenario: Mixed split mutates safely
- **WHEN** 测试创建 terminal + markdown + settings 的 mixed split tab
- **THEN** split、focus、move、close 和 equalize 操作保持 split tree 有效
- **AND** terminal runtime identity 不因非 terminal PaneSlot 操作重建

#### Scenario: Non-terminal command rejection tested
- **WHEN** control-plane 测试向 markdown 或 settings PaneSlot 发送 `terminal.send_text`
- **THEN** response 使用 stable unsupported-content error
- **AND** fake terminal runtime service 没有收到 delivery

#### Scenario: Terminal content identity survives movement
- **WHEN** 测试将 terminal ContentInstance 所在 PaneSlot 移动到另一个 tab 或重新 attach 视图
- **THEN** terminal runtime handle、scrollback、metadata 和 pending delivery 仍绑定到同一个 `content_id`

#### Scenario: Content rendering registry verified
- **WHEN** renderer registry 收到 terminal、markdown 和 settings content descriptor
- **THEN** 测试或 review checklist 确认每个 kind 路由到对应 renderer 或 bounded unavailable surface

#### Scenario: Visual evidence covers mixed content
- **WHEN** content-container UI implementation 标记完成
- **THEN** maintainers 可以检查 running-app screenshot 或记录，确认 light-mode shell 中存在混合 content tab/split，且 sidebar、toolbar、pane title bar 和 terminal-first chrome 保持一致
