# Human in the End：当 Agent 不再需要你盯着每一步

## 引言

“Human-in-the-Loop”（HITL）已经成为 AI 系统设计的行业共识——在关键节点引入人类审批，确保安全可控。但随着 Agent 向长时间自主运行演进，一个根本性的矛盾浮出水面：如果人类必须审批每一步，那 Agent 和一个带确认按钮的工作流有什么区别？

Anthropic 最近发布的一项[研究](https://www.anthropic.com/research/measuring-agent-autonomy)揭示了一个有趣的现实：Claude Code 用户中，新用户约 20% 的会话使用完全自动审批模式；而当使用超过 750 次后，这个比例飙升到 40% 以上。与此同时，最长运行的 Agent 会话时长从 2025 年 10 月的不到 25 分钟，翻倍增长到 2026 年 1 月的超过 45 分钟。用户正在用脚投票：他们越来越信任 Agent，越来越不想逐步审批。

本文提出 **Human-in-the-End（HITE）** 作为下一代 Agent 交互范式。这不是一个营销口号，而是一个需要被严肃定义的架构概念。它的核心主张是：人类角色的本质变化不在于”出现的位置”，而在于”承担的职责”——从过程审批者（Operator）升级为结果拥有者（Owner）。

最后，本文结合 [Alan](https://github.com/realmorrisliu/Alan)（一个基于 AI Turing Machine 隐喻构建的 Agent 运行时）的现有架构，推演 HITE 范式如何落地为具体的系统设计。

> 实施落地说明：本文是理念文档；对应的可执行规范已拆分到 `docs/spec/`（`kernel_contract`、`execution_model`、`governance_boundaries`、`memory_architecture`、`compaction_contract`、`app_server_protocol`）。

## Part 1: Human-in-the-Loop (HITL) 的本质与重要性

**Human-in-the-Loop（HITL）** 是一种 **“人类参与决策闭环”** 的系统设计模式，指的是 AI 或自动化系统在关键环节必须引入人类进行审核、纠正、反馈或最终决策。其本质不是“人工操作”，而是人类作为系统的一部分（闭环的一环）参与模型行为控制和优化。

### 自动化系统的三个层级

1. **Fully Automated（完全自动化）：** AI 自主完成所有操作（如自动发邮件、审批等）。优势是高效，但风险较高，错误不可控（例如产生幻觉）。
2. **Human-in-the-Loop（HITL）：** AI 提供建议，人类确认后再执行，人类扮演守门人（Gatekeeper）的角色。
3. **Human-on-the-Loop（HOTL）：** AI 自动执行，人类仅负责监控，只有在出现问题时才介入干预（如 L2 级别智能驾驶）。

### 为什么 HITL 很重要？

大语言模型（LLM）本质上是基于概率生成的（Probabilistic）而非绝对确定性的（Deterministic）。因此，在医疗、法律、金融、软件开发、内容安全等高风险、高敏感领域，必须引入 HITL 机制以控制损失。

典型的 AI Agent 场景应用：
- **代码审查 Agent（Code Review Agent）：** Agent 自动分析代码变更，检测潜在的 bug、安全漏洞或性能回退。由于误判可能导致生产事故，系统需将检测结果交由人类工程师进行审批（Approve / Reject / Edit），确认后再继续合并到主分支。
- **内容审核 Agent（Content Moderation Agent）：** Agent 负责扫描用户生成的内容，识别违规信息并决定处置方式（如删除、限流、标记）。为避免误判正常内容或产生过度审查，处置决策在执行前需由人类审核员过目确认。

### HITL 的三种常见技术架构表现

1. **Approval Node（审批节点）：** （最常见，如 LangGraph 等工作流）Agent 在关键步骤（如合并代码到主分支）前暂停，等待人类操作批准。
2. **Review Queue（审核队列）：** 任务状态暂时标记为等待审核，统一在系统 Inbox 等待人工集中处理。
3. **Tool Permission（工具提权）：** （如 Claude Code）当 AI 意图调用高危命令权限（如 `rm -rf`）时，需向人类申请确认。

### HITL 容易被忽视的核心价值：反馈学习（Feedback Learning）

HITL 不仅仅是为了防止出错和保证安全闭环，其更重要的是带来了**反馈学习（Feedback Learning）**的机会。人类的每一次纠正纠偏都是极具价值的数据。当人为标记“某处代码被误判为安全漏洞”时，意图被记录成数据，可以协助底层排序模型（Ranking Model）自我学习进化。这就是隐式的基于人类反馈的强化学习（Implicit RLHF）。

真正的顶级 HITL 不一定必须在前端表现为确认按钮，还可以是**隐式反馈（Passive HITL）**。用户主动操作（如删除了部分列表页数据），系统被动收集为负面反馈，这也是无痛的 Human in the loop。

---

## Part 2: 必然趋势：从 Human-in-the-Loop 走向 Human-in-the-End

随着 Agent 本身的能力扩张和工程体系成熟，越来越多的工作流会演变为长时间周期的独立运行模式。由此交互模式会逐步向 **Human-in-the-End（HITE）** 靠拢。

在传统的 HITL 模式中，**人被定义为”操作者（Operator）”**。系统的运行高度依赖人的频繁确认：批准一次 API 调用、确认一封邮件发送、审阅一次命令行执行。当 Agent 的能力从”一问一答”扩展到”连续运行数小时甚至数天”的长任务时，这种模式的矛盾变得不可调和：

- **审批疲劳（Approval Fatigue）：** 人类点了太多次确认，以至于不再真正阅读审批内容，安全机制沦为形式。David Farrell 在[《The Unsupervised Agent Problem》](https://blog.dnmfarrell.com/post/the-unsupervised-agent-problem/)中精确描述了这一现象。Anthropic 的数据也印证了这一点：经验丰富的 Claude Code 用户中，超过 40% 选择了完全自动审批。
- **带宽天花板：** 每次审批都是对人类的同步阻塞调用，Agent 的并发能力被人类的注意力带宽锁死。OpenAI 内部的 [Harness Engineering](https://openai.com/index/harness-engineering/) 团队报告了一个极端案例：他们经常看到单次 Codex 运行持续超过 6 小时——通常是在人类睡觉的时候。
- **角色错配：** 人类被迫评估低层级的执行细节（这个文件该不该写？这条命令该不该跑？），而他们真正的价值在于高层级的判断（方向对不对？结果达标了吗？）。Harness 团队的实践给出了一个更好的答案：他们将人类审查逐步让位给 agent-to-agent review，三名工程师用五个月时间驱动 Codex 完成了约 [1,500 个 PR、近百万行代码](https://openai.com/index/harness-engineering/)——没有一行是人类手写的。

Agent 越强，逐步监督就越没有意义。真正的 Agent 时代，我们必然走向 HITE。但这绝不意味着”完全无人监管的失控自动化”，人并非真的”只在最后出现”。HITE 的核心是人类角色的转变：从**”过程审批者”**跃升为**”结果拥有者（Owner）”**。

在 Alan 所倡导的 HITE 范式中，整个流转过程可以被重新定义为：

**Human Defines → Agent Executes → Human Owns**

这不是纯理论。OpenAI 的 Harness 团队在实践中已经验证了这个模型——他们的核心原则就是[“人类掌舵，智能体执行”](https://openai.com/index/harness-engineering/)。人类定义目标和约束，Agent 端到端地完成从编码到测试到部署的全流程，只在需要判断时升级给人类。

1. **Human Defines (Boundary Setting):** 人类在任务之前（Before）甚至运行时（During），负责设定目标（Goal）、预算约束、风控策略（Policy）以及不可逆动作的红线。人类定规则而非盯过程。
2. **Agent Executes:** 引擎依靠长任务机制与自我反思在沙盒边界内自动处理所有过程执行（Operational work）。正常的流程完全静默运作。
3. **Human Owns (Accountability/Exception Handling):** 人类最终为结果（Outcome）承担责任。系统仅在遇到策略越界（如超预算、触发高危操作等”异常”）时，才触发人类干预，这就是所谓的**基于异常的人类介入（Exception-Driven Human Oversight）**。

### 为什么人类永远不会完全消失

HITE 容易被误读为”完全自主的 AI，不需要人类”。人类不可替代的原因不是技术局限，而是结构性的：

- **法律责任（Accountability）：** 如果 Agent 签了一份有问题的合同或者把有 bug 的代码推到生产环境，承担法律和职业后果的是人类。系统中必须始终存在一个可问责的人——只是不需要在每一步都出现。
- **战略判断：** AI 可以优化（optimize），但不能定义目的（define purpose）。”要不要进入某个市场”不是优化问题，而是涉及风险偏好、文化和品牌的判断——这些无法被概率分布捕获。
- **不可量化价值：** 长期关系、信任网络、行业直觉——这些以模型无法复制的方式影响着决策。

将 HITL 升级为 HITE，是在工程上把人类从繁杂的执行步骤约束中解放出来，将关注点锚定在目标委派与业务问责上。

---

## Part 3: 面向未来的 Alan 架构发展推演

根据以上演进逻辑，对于 [Alan](https://github.com/realmorrisliu/Alan)（一个面向 long-running agent 的通用运行时）由于当前已经拥有 “Stateless AgentConfig + Stateful Workspace + Bounded Session” 解耦，且引入了 rollback / replay / approval / sandbox 的强大底盘，下一步想要进化到 “Outcome-driven 自治执行系统”，最亟需补充的设计建议如下：

### 1. 落地“提交边界”（Commit Boundaries）与策略即代码（Policy-as-Code）

如果 HITE（Human in the End）不再是一句营销口号，那它在底层协议上就必须具体化为 **Commit Boundaries（提交边界）**。

在传统的审批流中，拦截往往死板绑定于特定的工具调用。而在一个健壮的自治系统中，大多数日常操作（如读取文档、本地计算、试错尝试）应当完全放行。人类的介入必须被收敛、聚焦于那些真正不可逆的关键性业务关口：例如对外合同签发、真实资金打款、Git push 到生产分支。这就是运行时的“End Checkpoint”。

这些“提交边界”需要被定义为声明式的**策略即代码（Policy-as-Code）**。
- **动态风险评估容器：** 拦截与审批的触发不再死板地基于单个动作，而是基于动态上下文（如：操作范围是否越权、支出费用是否逼近水位线、代码变更行数是否过大），实现风控策略的显式左移。
- **将“异常”升格为一等公民：** Autonomous Agent 的核心不在于平顺时跑多快，而是遇到不可信地步时的平稳降级（Fallback）。正常业务静默运行；只有在触发“Commit Boundary”或发生偏离安全基准的异常时，系统才会抛出事件要求人类 Owner 接管介入。

### 2. 长时间任务核心：跨越上下文引入 Task/Job 维度

目前 Session 天然受制于上下文窗口的限制与存活生命周期管理，未来面临真正意义的长周期的 Agent 处理时，需要新增一层封装跨 Session 生命周期的调度单元：
- **Task/Job：** 作为业务最高层实体，携带全生命周期的 Goal、SLA 标准及资源硬约束和 Owner 身份。
- **Run：** 支持跨天轮转、任务自动流转重试、指数级回退（Backoff）调度的一次任务执行。
- **Session：** 退化为单个跑批动作的承载计算窗口。

核心难题是连续性。当新 Session 接手一个 Task 时，它需要足够的上下文来继续推进，而不是从头推导。这意味着需要结构化的交接产物——不是”这是对话历史”，而是”这是当前状态、已尝试的方案、剩余的工作”。

该分层抽象能天生满足未来长后台运行、无感队列处理以及”仅呈交最终结果”等面向过程免疫的诉求。

### 3. 可重放不仅是日志展示：核心化幂等性与产出审计链

要把 Rollout 从纯展示型日志（日志溯源）演进为具备真实侧重回放能力的技术体系：
- **幂等与安全重放（Idempotency）：** 让所有的 Tool Call 都持有 Idempotency Key 并在引擎层拦截保障副作用网络请求，回放模型里必须能够做到区分“历史拦截重载”和“强制重置执行”。
- **强化证据审计链（Provenance）：** “Outcome 结果”必须是附带防篡改溯源的。能够清晰追溯每级处理被哪项政策（Policy）放行，是参考了哪些源头数据进行推导的。这是消除人类全过程围观恐惧症的最后一片拼图。

### 4. 系统边界：用 Skills 编排，将 MCP 降级为底层工具

现在的行业趋势往往倾向于围绕 MCP（Model Context Protocol）构建 Agent。但从工程架构的长远来看，MCP 的设计实际上是违背 Unix 哲学的：

**为什么 MCP 不符合 Unix 哲学？**
Unix 哲学的核心是“一个程序只做一件事，并把它做好”，以及“程序之间通过文本流进行通信”。
MCP 反其道而行：它引入了一套复杂的 Client/Server 架构、握手协议和状态维护机制。为了让 Agent 能够使用一个简单的工具，开发者必须把工具包装成一个重型的 RPC 服务器。这增加了不必要的系统实体，导致系统过度设计（Over-engineering），违背了”如无必要，勿增实体”的原则。

OpenAI 自己也踩了这个坑。在构建 Codex 的 [App Server](https://openai.com/index/unlocking-the-codex-harness/) 时，他们最初尝试把 Codex 作为 MCP 服务器发布，但很快发现”难以维护 MCP 语义”，最终转向了自己的 JSON-RPC 协议。当需要支持丰富的 Agent 交互——线程生命周期、流式进度、审批中断——MCP 的抽象层级就不够用了。

**什么才是契合 Unix 哲学的 Agent 架构？**
真正的解法不在于制定更复杂的协议，而在于回归最基础的文本组合：
- **微型 CLI 工具：** 开发者继续用最顺手的语言写命令行工具（CLI），这些工具从标准输入（stdin）读取纯文本，并通过标准输出（stdout）返回纯文本。没有任何额外的 Server 或协议负担。
- **Skills 作为管道（Pipeline）：** 通过编写 Markdown 格式的 Skills，用人类自然语言将这些孤立的 CLI 工具像 Unix 管道一样串联起来。Agent 阅读 Skill（业务流程），然后通过标准的 `bash` 环境去调度各个小工具。

在这种架构中，MCP 和 OpenAPI 只是众多“原子执行器”中的一种；而 Skills 才是负责编排的“逻辑控制器”。将 MCP 降维，只作为在 Skills 内部被按需调用的底层设施。

具体来说，这种设计能解决以下四个真实的痛点：

#### 4.1 控制上下文，减少大模型幻觉
如果把几十个服务、成百上千个 Tools 的 Schema 全部塞进大模型的全局上下文中，极度浪费 Token，还会严重稀释关注点，导致模型频繁乱用工具。
**Skills 的隔离作用：** 像 `resolve_ticket.md` 这样的 Skill 相当于一个上下文隔离罩。模型不需要在全局看到所有工具，它只需要加载当前匹配的 Skill。在 Skill 内部，再去精确定向地调用当前任务需要的那一两个工具。按需加载，用完即弃。

#### 4.2 终结工具链的“组合黑盒”
完全依赖模型去“顿悟”如何把工具 A 的输出喂给工具 B，在现实中死板的 API 接口面前出错率极高。
**Skills 的硬连接：** Markdown 形式的 Skills 本质上就是预定义的组合逻辑。用自然语言明确写出：“第一步调用 A，第二步提取 `user_id`，第三步按特定格式传给 B”。Skills 直接充当了数据转换节点和流程控制单元，极大收敛了发散和试错的可能。

#### 4.3 状态与鉴权（Auth）的解耦
企业应用中存在复杂的登录与鉴权状态。在 Agent 框架层面用一套巨型协议去统一解决跨系统 Auth 是很脆弱的。
**自治的鉴权体系：** 符合 Unix 哲学的工具应该自己管理自己的状态。比如 `stripe cli`，它可以通过自己的命令完成 `login` 甚至拉起浏览器授权。Skills 里只需要注明：“执行特定操作前先运行 `stripe login`”。即使目前可能还需要人类输入一下验证码（HITL），未来也可以交给专门的鉴权子 Agent 来接管。整个流转过程中，核心的 Agent 运行时（如 Alan）始终保持纯粹的无状态（Stateless）。

#### 4.4 开发者中心，拒绝所有权倒挂
MCP 是一种平台思维，要求各个工具开发者去适应并在本地维护一层符合特定大模型厂商（如 Anthropic）要求的服务端实现。
**控制权回归：** 彻底的 Unix 架构把控制权交还给了开发者。你可以用 Go/Python/Rust 开发极简的内部 CLI 工具。完成之后，随手写一个 `SKILL.md`（或者直接提供 README）告诉大模型即可使用。不需要拉起常驻的 Server 进程，不需要处理跨语言的 RPC 依赖。用最基础的文本流串接引擎，这才是健康且低维护成本的开发者体验（DX）。

### 总结：下一步的优先级

如果要让 Alan 在企业级 long-running agent 环境中站稳脚跟，**“去肥增瘦”**是核心原则：
1. **构建 Task/Job 抽象**，打破单次 Session 的局限。
2. **落地 Policy-as-Code**，将人工点按确认升级为基于规则和风控的异常接管。
3. **实现执行的 Idempotency 及 Provenance 重放**，保证动作的幂等和可审计，这是建立企业信任的基础。
4. **坚持 Unix 哲学：用业务流程说明书（Skills）去指挥 Agent，把庞杂的工具库和 MCP 隐式降级为纯粹的命令行执行端。**
