# 回滚方案设计

> UtopOS 回滚系统：多层回滚架构 — 包管理器 / btrfs + 文件系统层，基于全程文件监控。

## 一、问题

包管理器只能回滚自己管理的变更。安装过程中可能出现：

| 问题 | 包管理器能回滚？ |
|------|:---:|
| 升级了 nginx 版本 | ✅ yum undo |
| 修改了已有配置文件 | ❌ 通常不能 |
| 权限被改了 | ❌ |
| 创建了临时文件/日志 | ❌ |
| 往无关目录写入东西 | ❌ |

**只靠包管理器回滚是不够的。** 需要更强大的回滚层来覆盖残余。

## 二、多层回滚架构

```
evo-rollback nginx --backend rpm --revert
│
├── Layer 1: 包管理器 / btrfs 回滚
│   ├── rpm:    yum history undo <txn>            (包级别)
│   ├── conda:  conda install --revision <rev>    (包级别)
│   └── btrfs:  btrfs subvolume snapshot 恢复     (文件系统级别, 原子操作)
│
├── Layer 2: 文件系统回滚（基于监控快照）
│   ├── 对比 before/after 快照
│   ├── 删除安装时新增的文件
│   ├── 恢复被修改的文件（从备份）
│   ├── 恢复被删除的文件（从备份）
│   └── 恢复被改的权限
│
└── 输出两层合并报告
```

### 关于 nix

nix 自带 generation 回滚机制，无需额外封装：

```bash
# nix 用户直接使用原生命令
nixos-rebuild switch --rollback          # 回滚到上一代
nixos-rebuild switch --to <generation>   # 回滚到指定代
```

UtopOS 不为 nix 提供回滚 wrapper，避免与原生机制冲突。

## 三、btrfs 回滚方案

### 原理

btrfs 的 COW (Copy-on-Write) 机制天然支持快照：

```
安装前 ──→ btrfs subvolume snapshot -r / @pre-nginx    (只读快照)
安装中 ──→ yum install nginx                            (正常安装)
回滚时 ──→ btrfs subvolume snapshot -r / @rollback-bak (保险快照)
        ──→ btrfs subvolume snapshot @pre-nginx @restore (可写恢复)
```

### 优势

| 特性 | 说明 |
|------|------|
| **零开销快照** | COW 机制，快照不复制数据，仅记录差异 |
| **原子操作** | 快照创建/恢复是原子的，不存在中间状态 |
| **无遗漏** | 任何文件变更都能捕获，不限于包管理器管理的文件 |
| **秒级回滚** | 大文件系统也能瞬间完成 |
| **可验证** | `btrfs subvolume show` 验证快照完整性 |

### 要求

- 根分区（或目标分区）使用 btrfs 文件系统
- 安装 `btrfs-progs`（多数发行版默认包含）
- 足够磁盘空间存放快照（COW 机制下通常很小）

### 快照策略

```bash
# 安装前: 创建只读快照
btrfs subvolume snapshot -r / $EVO_HOME/btrfs-snapshots/pre-install-nginx-20260413

# 安装后: 创建只读快照 (用于 Layer 2 diff)
btrfs subvolume snapshot -r / $EVO_HOME/btrfs-snapshots/post-install-nginx-20260413

# 回滚时:
# 1. 保险快照 (保留回滚前状态)
btrfs subvolume snapshot -r / $EVO_HOME/btrfs-snapshots/rollback-from-20260413
# 2. 从 pre-install 快照恢复
btrfs subvolume snapshot $EVO_HOME/btrfs-snapshots/pre-install-nginx-20260413 \
                         $EVO_HOME/btrfs-snapshots/restore-20260413
```

### 回收旧快照

快照会逐渐累积，定期清理：

```bash
# 保留最近 10 个 pre-install 快照
ls -dt $EVO_HOME/btrfs-snapshots/pre-install-* | tail -n +11 | xargs btrfs subvolume delete
```

## 四、监控如何支撑回滚

每次安装自动执行三个动作：

```
evo-install nginx --backend rpm --monitor /etc
│
├── 1. evo-snapshot /etc (before)
│   └── 生成 before.manifest
│
├── 2. rsync /etc → $EVO_HOME/backups/nginx-pre-install-xxx/
│   └── 完整文件副本，回滚时恢复内容用
│
├── 3. 包管理器执行安装
│
├── 4. evo-snapshot /etc (after)
│   └── 生成 after.manifest
│
└── 5. evo-diff before after → 生成报告
```

btrfs 后端额外在步骤 1 前创建子卷快照。

## 五、工具链

| 脚本 | 职责 | 在回滚中的角色 |
|------|------|-------------|
| `evo-snapshot` | 拍摄快照 | 安装前后各拍一次 |
| `evo-diff` | 对比快照 | 找出哪些文件变了 |
| `evo-monitor` | 包装命令 | 一条命令完成 安装+监控 |
| `evo-install` | 包管理器安装 | 安装时自动快照+备份 |
| `evo-rollback` | **多层回滚入口** | Layer1 包管理器/btrfs + Layer2 文件系统 |
| `evo-revert` | 文件系统回滚 | Layer2 的具体执行者 |
| `evo-rollback-plan` | 回滚预览 | dry-run 两层，不做实际操作 |
| `evo-fence` | 实时监控 | 排查第三方安装 |

## 六、命令速查

### 安装（自动监控）

```bash
# rpm 安装
evo-install nginx --backend rpm --monitor /etc

# btrfs 安装（自动创建子卷快照）
evo-install nginx --backend btrfs --monitor /etc

# conda 安装
evo-install pandas --backend conda --monitor ~/miniconda3/envs
```

### 回滚

```bash
# rpm 两层回滚
evo-rollback nginx --backend rpm --revert

# btrfs 回滚（Layer1 用 btrfs 快照 + Layer2 文件系统补漏）
evo-rollback nginx --backend btrfs --revert

# conda 回滚
evo-rollback pandas --backend conda --revert

# 回滚到指定目标（事务 ID / 快照路径 / revision）
evo-rollback nginx --backend rpm --to 15 --revert
evo-rollback nginx --backend btrfs --to $EVO_HOME/btrfs-snapshots/pre-install-nginx-20260413 --revert

# dry-run 预览
evo-rollback nginx --backend btrfs --revert --dry-run
```

### 回滚预览

```bash
# 查看回滚会做什么（不执行）
evo-rollback nginx --backend btrfs --plan
```

输出示例:
```json
{
  "status": "ok",
  "action": "plan",
  "backend": "btrfs",
  "layer1": {
    "layer": "btrfs",
    "action": "btrfs subvolume snapshot → pre-install-nginx-20260413",
    "latest_snapshot": "/root/.evo/btrfs-snapshots/pre-install-nginx-20260413"
  },
  "layer2": {
    "stats": {"created": 3, "deleted": 1, "modified": 5},
    "changes": [...]
  }
}
```

### 手动监控任意命令

```bash
# 监控非包管理器的安装
evo-monitor /opt --name myapp-install -- bash install.sh

# 实时监控
evo-fence /etc --timeout 300 &
yum install something
# 检查 fence 日志
```

## 七、数据目录结构

```
$EVO_HOME/
├── snapshots/          # 快照 manifest (Layer 2 用)
│   ├── nginx-pre-install-20260413.manifest
│   ├── nginx-post-install-20260413.manifest
│   └── ...
├── btrfs-snapshots/    # btrfs 子卷快照 (Layer 1 btrfs 后端)
│   ├── pre-install-nginx-20260413/    (只读子卷快照)
│   ├── post-install-nginx-20260413/
│   ├── rollback-from-20260413/        (回滚保险快照)
│   └── restore-20260413/              (恢复后的可写子卷)
├── backups/            # 文件副本（Layer 2 恢复内容用）
│   ├── nginx-pre-install-20260413/
│   │   ├── nginx/
│   │   └── ...
│   └── ...
├── reports/            # 安装/回滚报告
│   ├── nginx-install-20260413.json
│   ├── nginx-rollback-20260413.json
│   └── ...
├── history/            # 事务历史 (JSONL)
│   └── nginx.jsonl
└── ...
```

## 八、预期变更清单格式

通过 `--expected` 指定本次安装预期变更的路径：

```jsonl
{"path": "nginx/nginx.conf"}
{"path": "nginx/conf.d/"}
{"path": "nginx/logs/"}
```

不在清单中的变更会被标记为 unexpected，报告 `verdict: WARNING`。

## 九、依赖

| 组件 | rpm 后端 | conda 后端 | btrfs 后端 |
|------|:---:|:---:|:---:|
| `sha256sum` / `find` / `stat` (coreutils) | ✅ | ✅ | ✅ |
| `rsync` | ✅ | ✅ | 可选 |
| `python3` | ✅ | ✅ | ✅ |
| `yum` / `dnf` | ✅ | ❌ | ❌ |
| `conda` | ❌ | ✅ | ❌ |
| `btrfs-progs` | ❌ | ❌ | ✅ |
| btrfs 文件系统 | ❌ | ❌ | ✅ |

## 十、后端选择指南

```
根分区是 btrfs?
├── 是 → --backend btrfs (推荐：原子回滚，零开销)
│         Layer 1 用 btrfs 快照 + Layer 2 文件系统补漏
│
└── 否 → 用什么包管理器?
          ├── yum/dnf → --backend rpm
          └── conda   → --backend conda
```

**btrfs 是最强回滚方案**：快照零开销、原子操作、无遗漏。如果你的系统用了 btrfs，这是首选。
