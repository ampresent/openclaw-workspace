# 回滚方案设计

> UtopOS 回滚系统：两层回滚架构，包管理器层 + 文件系统层，基于全程文件监控。

## 一、问题

包管理器只能回滚自己管理的变更。安装过程中可能出现：

| 问题 | 包管理器能回滚？ |
|------|:---:|
| 升级了 nginx 版本 | ✅ yum undo |
| 修改了已有配置文件 | ❌ 通常不能 |
| 权限被改了 | ❌ |
| 创建了临时文件/日志 | ❌ |
| 往无关目录写入东西 | ❌ |

**只靠包管理器回滚是不够的。** 需要第二层来覆盖残余。

## 二、两层回滚架构

```
evo-rollback nginx --revert
│
├── Layer 1: 包管理器回滚
│   ├── nix:  nixos-rebuild switch --rollback
│   ├── rpm:  yum history undo <txn>
│   └── conda: conda install --revision <rev>
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

## 三、监控如何支撑回滚

每次安装自动执行三个动作：

```
evo-install nginx --monitor /etc
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

回滚时，`evo-rollback --revert` 自动找到这些快照和备份：

```
evo-rollback nginx --revert
│
├── 找到 nginx-pre-install-xxx.manifest (before)
├── 找到 nginx-post-install-xxx.manifest (after)
├── 找到 backups/nginx-pre-install-xxx/ (文件副本)
│
├── Layer 1: yum history undo
│   └── 包管理器回滚自己管理的变更
│
├── Layer 2: evo-revert /etc --from before --to after
│   ├── diff 找出变更
│   ├── created → 删除
│   ├── deleted → 从 backups/ 恢复
│   ├── modified → 从 backups/ 恢复
│   └── permission → 恢复 mode
│
└── 合并报告: {"layer1":"ok","layer2":"ok","reverted":5}
```

## 四、工具链

| 脚本 | 职责 | 在回滚中的角色 |
|------|------|-------------|
| `evo-snapshot` | 拍摄快照 | 安装前后各拍一次 |
| `evo-diff` | 对比快照 | 找出哪些文件变了 |
| `evo-monitor` | 包装命令 | 一条命令完成 安装+监控 |
| `evo-install` | 包管理器安装 | 安装时自动快照+备份 |
| `evo-rollback` | **两层回滚入口** | Layer1 包管理器 + Layer2 文件系统 |
| `evo-revert` | 文件系统回滚 | Layer2 的具体执行者 |
| `evo-rollback-plan` | 回滚预览 | dry-run 两层，不做实际操作 |
| `evo-fence` | 实时监控 | 排查第三方安装 |

## 五、命令速查

### 安装（自动监控）

```bash
# 基础用法 — 安装时自动快照+备份
evo-install nginx --backend rpm --monitor /etc

# 带预期变更清单 — 清单外变更标记 WARNING
evo-install nginx --monitor /etc --expected nginx-expected.jsonl
```

### 回滚

```bash
# 完整两层回滚
evo-rollback nginx --revert

# 只做包管理器回滚（不恢复文件）
evo-rollback nginx

# 回滚到指定事务
evo-rollback nginx --to 15 --revert

# dry-run 预览
evo-rollback nginx --revert --dry-run
```

### 回滚预览

```bash
# 查看回滚会做什么（不执行）
evo-rollback nginx --plan
```

输出示例:
```json
{
  "status": "ok",
  "action": "plan",
  "layer1": {"action": "yum history undo 15"},
  "layer2": {
    "stats": {"created": 3, "deleted": 1, "modified": 5},
    "changes": [...]
  }
}
```

### 手动监控任意命令

```bash
# 监控一个非包管理器的安装过程
evo-monitor /opt --name myapp-install -- bash install.sh

# 实时监控
evo-fence /etc --timeout 300 &
yum install something
# 检查 fence 日志
```

## 六、数据目录结构

```
$EVO_HOME/
├── snapshots/          # 快照 manifest
│   ├── nginx-pre-install-20260413.manifest
│   ├── nginx-post-install-20260413.manifest
│   └── ...
├── backups/            # 文件副本（回滚时恢复内容用）
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

## 七、预期变更清单格式

通过 `--expected` 指定本次安装预期变更的路径：

```jsonl
{"path": "nginx/nginx.conf"}
{"path": "nginx/conf.d/"}
{"path": "nginx/logs/"}
```

不在清单中的变更会被标记为 unexpected，报告 `verdict: WARNING`。

## 八、依赖

- `sha256sum` / `find` / `stat` (coreutils)
- `rsync` 或 `cp` — 文件备份
- `python3` — diff 和 JSON 处理
- `inotifywait` (inotify-tools) — 仅 evo-fence 需要

全部在主流 Linux 发行版默认安装（rsync 除外，apt/yum install rsync 即可）。
