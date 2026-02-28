# Alan Native Client (SwiftUI)

`clients/apple` 是 Alan 的原生 Apple 客户端基础工程，支持 macOS 和 iOS。

## 系统要求

- iOS 26+
- macOS 26+

客户端界面使用系统 Liquid Glass 组件（如 `.glass` / `.glassProminent`）。

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

默认会连接 `http://127.0.0.1:8090`，可在 UI 中修改。

## 当前功能

- 检查 daemon 健康状态（`/health`）
- 创建会话（`/api/v1/sessions`）
- 基础聊天闭环：
  1. 提交消息（`/api/v1/sessions/{id}/submit`）
  2. 拉取事件（`/api/v1/sessions/{id}/events/read`）
  3. 读取 `text_delta` 直到 `turn_completed`

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
