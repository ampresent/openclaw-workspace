# Evolution OS 设计白皮书 (v0.2)

## 一、项目愿景与核心哲学

Evolution OS 不是一个传统的二进制发行版，而是一个处于"持续编译与自我进化"状态的元操作系统。

### 三大核心信条

1. **源码即系统**：一切皆为源码。系统状态由本地 Git 仓库树和 Patch 栈定义。最终目标：对系统的任何修改（包括配置文件的改动）都必须体现为对源码仓库的一次 Commit 或 Patch（激进 Todo）。
2. **AI 辅助演进**：用户不再是功能的被动接收者，而是进化的**发起者**。系统通过本地小模型感知用户意图，并借助 **Claude Code** 作为核心引擎来修改源码、解决构建冲突。
3. **自举与自治**：基于 Rocky Linux，Evolution OS 能够编译和构建自身，且在大部分时间里保持**静默进化**，仅在需要决策时唤醒用户。

---

## 二、系统架构与生命周期

### 2.1 基础底座：Rocky Linux + Patch 管理

- 初始种子基于 Rocky Linux src.rpm 生成。
- 源码维护采用 **Patch 栈**模式：
  - Base 源码从 Rocky src.rpm 提取
  - 所有定制修改以 Patch 形式叠加
  - Patch 按功能/模块组织，支持独立回滚
- **自举验证**：系统必须能在已安装的 Evolution OS 上，仅使用本地源码仓库 + Patch 栈，重新编译出完全一致的操作系统镜像。

### 2.2 包管理与文件系统（源码驱动设计）

- **不允许裸二进制**：除 `/boot` 和救援内核外，`/usr` 下的任何可执行文件必须对应至少一个本地源码仓库和编译描述文件。
- **临时文件豁免**：`/proc`、`/sys`、`/run`、`/tmp` 等运行时状态目录不纳入源码管理范畴。
- **修改拦截机制（第一阶段）**：
  - 仅针对**软件包**进行源码级管理。
  - *激进 Todo*：参考 NixOS 的原子化设计，未来考虑拦截对 `/etc` 的直接修改，将其转化为对配置包源码的 Patch。

### 2.3 智能体：远程 AI 模型

#### 模型配置

Evolution OS 通过配置文件接入 AI 模型，默认复用本机已有的 OpenClaw 模型配置。

```toml
# .evo/config.toml
[ai]
provider = "xiaomi"
base_url = "https://api.xiaomimimo.com/v1"
model = "mimo-v2-pro"
api_key_env = "EVO_AI_API_KEY"    # 从环境变量读取密钥
# 或直接配置:
# api_key = "sk-..."
```

- **远程模型（必须）**：负责所有 AI 任务——阅读源码、生成补丁、解决冲突、意图识别
- **本地小模型（可选）**：如配置了 `local_model` 字段，用于低延迟的命令错误截获和意图分流

#### 安全边界

- **用户权限隔离**：Claude Code 和构建进程运行在独立用户下，无 root 权限
- 所有修改仅作用于源码树，通过标准构建流程产出

#### 工作流

1. 用户执行 `my_tool --new-feature` 报错
2. 系统捕获错误，调用远程模型分析："检测到缺失功能，是否修改源码以实现该目的？"
3. 用户确认后，系统将当前源码 Diff 和错误日志发送给远程模型
4. 模型返回 Patch，系统直接应用到本地源码树并触发构建

---

## 三、构建与升级策略

### 3.1 构建调度器

- **触发条件**：纯手动触发。系统绝不自动拉取上游更新。
- **资源限制**：优先针对服务器场景。构建任务默认使用所有空闲 CPU，调度器优先级为 Idle，确保 SSH 交互不受影响。
- **中断处理**：构建未完成时关机，丢弃当前构建产物。下次开机后需重新触发。

### 3.2 稳定性与回滚

- **稳定标记（Tag）**：`evo tag --create stable-2026-04-12` 对整个系统源码状态打标
- **救援系统**：
  - 复用本地已有工具链
  - 支持一键回滚到：昨日快照 / 用户指定 Tag 状态

---

## 四、用户交互与开发流

### 4.1 静默进化与 TUI 看板

- 默认模式：后台静默编译，不弹通知
- `evo status` TUI：当前构建进度、Claude Patch Diff 预览、上游 Rebase 冲突列表

### 4.2 "命令错误即开发"协议

**场景 A：参数缺失**
```
$ evolve-tool --render-gif
evolve-tool: unrecognized option '--render-gif'
[意图识别] 您希望输出 GIF 动图。当前源码支持 PNG。
是否分析 evolve-tool 源码以增加该参数？[Y/n]
```

**场景 B：Rebase 冲突自动解决**
```
$ evo rebase --ai
  → fetching upstream src.rpm...
  → applying new base source...
  → reapplying 3 patches...
    → 0001-custom-sched.patch ok
    → 0002-evo-hook.patch CONFLICT
    → 0003-another.patch ok
  rebase: upstream update (2 patches, 1 conflicts)
  → invoking AI conflict resolver...
  → resolving kernel...
  [AI suggests resolution steps...]
```

### 4.3 进化暂停开关

`evo freeze` → 系统退化为只读普通 Linux，关闭所有 AI 监听钩子和自动构建。

---

## 五、与开源社区的交互

1. **本地为王**：上游是否接受代码不影响本地运行
2. **手动推送**：不会自动向 GitHub/GitLab 提交 PR
3. **交互辅助**：回馈社区时辅助 `git format-patch` + 邮件草稿生成
4. **Rebase 辅助**：`evo rebase` 交互模式，AI 辅助解决冲突

---

## 六、技术待办

### 短期

- [ ] Rocky Linux src.rpm 提取流程
- [ ] Patch 栈管理工具
- [ ] `evo` CLI (Rust) 基础框架
- [ ] 构建调度器守护进程
- [ ] Claude Code 安全交互接口（用户权限沙箱）

### 激进愿景

- 原子级源码映射：修改 `/etc/hostname` → 拦截 → 转化为源码 Patch → 最小化重构建
- 任何 Evolution OS = 一段特定 Git Commit Hash 的编译投影
