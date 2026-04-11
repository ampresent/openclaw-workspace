# 技术架构

## 系统分层

```
┌─────────────────────────────────────────┐
│           evo CLI (Rust)                │  用户交互层
│   status / build / rebase / tag / freeze │
├─────────────────────────────────────────┤
│         调度器守护进程 (evo-daemon)       │  调度层
│   构建队列 / 资源调度 / 状态管理          │
├─────────────────────────────────────────┤
│        AI 集成层 (ai-integration)        │
│   ┌───────────┐  ┌──────────────────┐   │
│   │ 本地小模型 │  │ Claude Code 接口  │   │
│   │ (感知分流) │  │ (云端引擎)        │   │
│   └───────────┘  └──────────────────┘   │
├─────────────────────────────────────────┤
│          Patch 管理层                    │
│   Patch 栈 / Diff 工具 / 冲突检测        │
├─────────────────────────────────────────┤
│        构建系统 (build)                  │
│   Rocky src.rpm 解包 / 编译 / 打包       │
├─────────────────────────────────────────┤
│         源码仓库 (Git)                   │
│   Base 源码 + Patch 栈 + 构建描述        │
└─────────────────────────────────────────┘
```

## 关键设计决策

### 源码组织

```
/opt/evo/src/                    # 源码根目录
├── base/                        # Rocky src.rpm 原始源码
│   ├── kernel/
│   ├── glibc/
│   ├── systemd/
│   └── ...
├── patches/                     # Patch 栈
│   ├── kernel/
│   │   ├── 0001-custom-sched.patch
│   │   └── 0002-evo-hook.patch
│   ├── glibc/
│   └── ...
├── specs/                       # 构建描述文件 (RPM spec)
└── builds/                      # 构建产物 (gitignore)
```

### Patch 工作流

```
1. evo init <package>           # 从 src.rpm 提取源码
2. (修改源码)
3. evo patch create <package>   # 生成 patch 并入栈
4. evo build <package>          # 应用全部 patch 后构建
5. evo patch drop <N>           # 回滚到第 N 个 patch
```

### 安全模型

```
用户 (root)          → evo CLI 操作 / 系统管理
evo 用户 (uid)       → 构建进程 / Claude Code 操作
                      仅有 /opt/evo/src 的读写权限
                      无系统目录写权限
```

### 本地小模型集成（可选）

如配置了 `local_model`，用于：
- 截获 stderr，识别"命令不存在" / "参数错误"
- 生成意图识别提示（低延迟）
- 分流决策：直接回答 vs 启动远程模型开发流

未配置时，所有 AI 任务由远程模型处理。

### 远程 AI 模型集成

- OpenAI-compatible API (base_url + model + api_key)
- 读取 `.evo/config.toml` 的 `[ai]` 配置
- 支持的 AI 任务：
  - 源码分析：阅读 patch 上下文，解释变更
  - 补丁生成：根据错误日志 + 源码 diff 生成 patch
  - 冲突解决：分析 rebase 冲突，给出解决建议
  - 意图识别：分析命令错误，建议是否启动开发流
