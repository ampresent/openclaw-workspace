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

### 本地小模型集成

参考 small-model-lab 项目，本地模型负责：
- 截获 stderr，识别"命令不存在" / "参数错误"
- 生成意图识别提示
- 分流决策：直接回答 vs 启动 Claude Code 开发流

### Claude Code 集成

- 通过 API 或 CLI 调用
- 输入：源码 Diff + 错误日志 + 用户意图描述
- 输出：标准 Patch 格式
- 运行在 `evo` 用户下，沙箱隔离
