## ADDED Requirements

### Requirement: Pane layout operations are content-agnostic
The macOS shell SHALL treat split, focus, resize, equalize, pane lift, cross-tab move, and close pane as PaneSlot operations over the split layout tree, not as terminal-only operations.

#### Scenario: Split terminal pane with markdown target
- **WHEN** 用户在 terminal pane 旁创建 markdown split
- **THEN** alan 在同一个 tab 中插入新的 pane slot
- **AND** 新 PaneSlot 承载 markdown ContentInstance
- **AND** 原 terminal ContentInstance 的 runtime identity 保持连续

#### Scenario: Focus moves from terminal to settings pane
- **WHEN** 用户从 terminal pane 空间聚焦到同一 tab 内的 settings pane
- **THEN** shell focus 更新到 settings PaneSlot
- **AND** terminal runtime 保持后台存活，不接收 settings pane 的键盘输入

#### Scenario: Move mixed pane between tabs
- **WHEN** 用户将 markdown 或 settings pane 移动到另一个 tab
- **THEN** alan 保持该 PaneSlot 和 ContentInstance identity 连续
- **AND** source 和 target tab 的 split tree 都保持有效

### Requirement: Tab creation accepts content intent
创建 tab 或 split pane 时，macOS shell SHALL 接受 content intent，并在 intent 缺省时保持现有
terminal tab 行为。

#### Scenario: New terminal tab remains default
- **WHEN** 用户执行现有 New Terminal Tab 行为
- **THEN** alan 创建承载 `terminal` ContentInstance 的 tab
- **AND** 现有 keyboard/menu/command 行为保持兼容

#### Scenario: New settings tab opens
- **WHEN** 用户执行 Open Settings in Tab 行为
- **THEN** alan 在当前 space 创建或聚焦承载 canonical `settings` ContentInstance 的 tab
- **AND** sidebar tab row 使用用户可见设置标题，而不是 raw content ID

#### Scenario: New markdown tab opens
- **WHEN** 用户请求打开 markdown 文件为 tab
- **THEN** alan 创建承载 `markdown` ContentInstance 的 tab
- **AND** tab title 从文件名或 content title 派生

#### Scenario: Settings tab is singleton
- **WHEN** 用户再次执行 Open Settings in Tab 行为
- **THEN** alan 聚焦已存在的 settings ContentInstance 所在 PaneSlot
- **AND** alan MUST NOT 创建重复 settings tabs，除非未来 capability 明确引入多实例 settings

### Requirement: Sidebar and command routing understand content kind
Sidebar、toolbar、command input 和 menu routing SHALL 使用 content kind、title 和 capabilities
来展示和执行 tab/pane 操作，而不是把所有 PaneSlots 视为 terminal target。

#### Scenario: Sidebar lists mixed content tabs
- **WHEN** 一个 space 中存在 terminal、markdown 和 settings tabs
- **THEN** sidebar 使用各自用户可见标题和 restrained content affordance
- **AND** 默认 UI 不暴露 raw pane IDs、content IDs 或 renderer implementation names

#### Scenario: Command input resolves content-aware target
- **WHEN** 用户通过 command input 跳转到 markdown 或 settings pane
- **THEN** alan 聚焦对应 PaneSlot
- **AND** 不执行 terminal-specific focus side effect，例如请求 terminal host first responder
