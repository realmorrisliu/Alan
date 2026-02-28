# Alan Native Client (SwiftUI)

`clients/apple` 是 Alan 的原生 Apple 客户端工程，支持 macOS 和 iOS。

## 系统要求

- iOS 26+
- macOS 26+

## 目录结构

```text
clients/apple/
├── AlanNative.xcodeproj/
│   └── project.pbxproj
└── AlanNative/
    ├── AlanNativeApp.swift
    ├── ContentView.swift
    └── AlanAPIClient.swift
```

## 快速开始

1. 用 Xcode 打开 `clients/apple/AlanNative.xcodeproj`
2. 选择 `AlanNative` scheme
3. 选择运行目标：`My Mac` 或 iOS 模拟器/真机
4. 运行应用

默认连接 `http://127.0.0.1:8090`，可在 UI 中修改。

## 当前功能（v0.1）

### 桌面端（macOS）

- 侧栏式工作台布局（连接面板 + 会话列表 + 新会话配置）
- 会话管理：创建、切换、刷新、fork、删除
- 运行控制：interrupt / compact / rollback
- 聊天与 steering：`Op::Turn` 与 `Op::Input`
- 事件时间线：turn/tool/yield/warning/error
- Yield 交互：
  - confirmation（approve / reject / modify）
  - structured_input（表单回答）
  - custom/dynamic（手动 JSON resume）
- 历史恢复：会话切换时读取 `/read` 历史并持续 `/events/read` 增量同步

### 手机端（iOS）

- 远程控制优先布局（Chat / Timeline 双面板）
- 与桌面一致的核心控制能力：
  - 连接远端 daemon
  - 会话切换与消息提交
  - yield 审批/输入恢复
  - interrupt/compact/rollback/fork

## 协议与端点

客户端基于现有 `/api/v1/sessions/*` 兼容层：

- `GET /health`
- `GET /api/v1/sessions`
- `POST /api/v1/sessions`
- `GET /api/v1/sessions/{id}/read`
- `POST /api/v1/sessions/{id}/resume`
- `POST /api/v1/sessions/{id}/fork`
- `DELETE /api/v1/sessions/{id}`
- `POST /api/v1/sessions/{id}/submit`
- `GET /api/v1/sessions/{id}/events/read`

## 命令行构建

```bash
# macOS
xcodebuild \
  -project clients/apple/AlanNative.xcodeproj \
  -scheme AlanNative \
  -destination "generic/platform=macOS" \
  -derivedDataPath /tmp/alan-native-dd \
  build

# iOS
xcodebuild \
  -project clients/apple/AlanNative.xcodeproj \
  -scheme AlanNative \
  -destination "generic/platform=iOS" \
  -derivedDataPath /tmp/alan-native-ios-dd \
  build
```
