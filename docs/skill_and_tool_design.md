# Alan Skill & Tool 系统设计（已实施）

> **设计原则**: 精简内核，Skills 自给自足，No MCP
>
> **核心隐喻**: AI Turing Machine — core 是状态机，tools 是副作用，skills 是指令扩展
>
> **实施状态**: ✅ 已完成

## 1. 架构概览（已实施）

```
┌─────────────────────────────────────────┐
│           Skills (自给自足)              │
│  ┌─────────────────────────────────┐    │
│  │  my-skill/                      │    │
│  │  ├── SKILL.md                   │    │
│  │  ├── scripts/                   │    │
│  │  └── references/                │    │
│  └─────────────────────────────────┘    │
└──────────────┬──────────────────────────┘
               │ 通过 bash 调用
               ▼
┌─────────────────────────────────────────┐
│      7 Core Tools (alan-tools) ✅       │
│                                         │
│  read_file  write_file  edit_file       │
│  bash       grep        glob            │
│  list_dir                               │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│      Sandbox (Workspace-only) ✅        │
└─────────────────────────────────────────┘
```

## 2. 已实施的组件

### 2.1 Skill System ✅

**文件结构:**
```
crates/runtime/src/skills/
├── mod.rs        # 模块入口 + init() + list_skills()
├── types.rs      # Skill 类型定义
├── loader.rs     # SKILL.md 加载器
├── registry.rs   # Skill 注册表
└── injector.rs   # Prompt 注入器 ($skill-name 提取 + 注入)
```

**特性:**
- ✅ Markdown + YAML frontmatter
- ✅ 两级范围: Repo (`.alan/skills/`) > User (`~/.config/alan/skills/`)
- ✅ `$skill-name` 触发
- ✅ 渐进式披露 (SKILL.md + scripts/ + references/)
- ✅ 无内置 skills — 所有 skills 从磁盘加载

**使用示例:**
```bash
mkdir -p .alan/skills/my-skill
cat > .alan/skills/my-skill/SKILL.md << 'EOF'
---
name: My Skill
description: A custom skill
capabilities:
  required_tools:
    - read_file
    - bash
---

# Instructions

1. Read relevant files
2. Process with bash
3. Report results
EOF
```

### 2.2 Tool System ✅

**Trait 定义在 core, 实现在 alan-tools:**
```
crates/runtime/src/tools/       # Tool trait + ToolRegistry
crates/tools/src/lib.rs      # 7 工具的实现 (alan-tools crate)
```

`alan-runtime` 只定义 `Tool` trait 和 `ToolRegistry`。具体工具实现（文件读写、bash 等）在独立的 `alan-tools` crate 中，保持 core 的通用性。

### 2.3 7 Core Tools ✅

| 工具         | 状态 | 描述                                  |
| ------------ | ---- | ------------------------------------- |
| `read_file`  | ✅    | 读取文件，支持 offset/limit，图片检测 |
| `write_file` | ✅    | 写入文件，自动创建父目录              |
| `edit_file`  | ✅    | 搜索替换编辑                          |
| `bash`       | ✅    | 执行 shell 命令                       |
| `grep`       | ✅    | 递归正则搜索                          |
| `glob`       | ✅    | 文件路径匹配                          |
| `list_dir`   | ✅    | 目录列表，目录优先排序                |

### 2.4 Sandbox ✅

- Workspace-only 路径检查
- 自动 canonicalize 处理符号链接
- 支持新文件创建（检查父目录）
- 不依赖 Landlock/Seatbelt

## 3. 与 AI Turing Machine 的关系

在 AI Turing Machine 隐喻中，Tools 和 Skills 扮演不同的角色：

| 概念        | TM 隐喻  | 说明                                                   |
| ----------- | -------- | ------------------------------------------------------ |
| **Tools**   | 副作用   | 状态机对外部世界的操作接口                             |
| **Skills**  | 指令扩展 | 动态注入到 prompt 的行为指令，改变 transition function |
| **Sandbox** | 边界约束 | 限制副作用的作用范围                                   |

Core 不知道具体有哪些工具实现。`ToolRegistry` 是抽象接口，具体实现由外层（`alan-tools`）注入。这保持了 core 的通用性 — core 是一个纯粹的状态转换引擎。

## 4. Skill 扩展能力的方式

### 4.1 使用 bash + curl

```yaml
---
name: API Client
description: Call external API
capabilities:
  required_tools: [bash]
---

When asked to fetch data:
1. Run: `curl -s "$API_ENDPOINT"`
2. Parse JSON response
3. Present results
```

### 4.2 自带脚本

```
.alan/skills/code-analyzer/
├── SKILL.md
└── scripts/
    └── analyze.py
```

## 5. 测试

```bash
cargo test -p alan-tools         # 工具实现测试
cargo test -p alan-runtime skills   # Skill 系统测试
cargo test -p alan-runtime tools    # Tool registry 测试
```

## 6. 总结

### ✅ 已完成

1. **Skill System** — Markdown + YAML, Repo/User 两级, $skill 触发
2. **7 Core Tools** — 独立 `alan-tools` crate, 通过 `ToolRegistry` 注入
3. **Sandbox** — Workspace-only, 无外部依赖
4. **AI TM 对齐** — Core 只定义接口, 具体实现注入

### 🎯 设计目标达成

- ✅ **精简内核**: Core 不含工具实现，只有 trait 和 registry
- ✅ **Skills 自给自足**: 自带脚本，通过 bash 调用
- ✅ **No MCP**: 无外部协议依赖
- ✅ **简单可靠**: 路径检查替代 OS 沙盒
