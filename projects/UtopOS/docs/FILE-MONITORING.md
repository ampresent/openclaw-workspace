# 文件变更监控系统设计

> UtopOS 监控层：在安装包、执行变更时全量跟踪文件系统改动，区分预期/异常变更，支持回滚。

## 一、问题

包管理器安装/升级时，可能对系统做非预期的修改：
- 创建未知配置文件
- 修改已有文件权限
- 删除不该删的文件
- 往无关目录写入临时文件

没有监控就没有回滚的依据。

## 二、架构

```
安装流程（包装后）:

  evo-snapshot (before)
       │
       ▼
  包管理器命令执行
       │
       ▼
  evo-snapshot (after)
       │
       ▼
  evo-diff (对比)
       │
       ├── 有 expected 清单 → 交叉检查 → 标记异常
       │
       ▼
  生成报告 (JSON)
       │
       ├── verdict: OK / WARNING / CMD_FAILED
       │
       ▼
  需要回滚？
       │
       ▼
  evo-revert --from before --to after
```

### 工具链

| 脚本 | 职责 | 输入 | 输出 |
|------|------|------|------|
| `evo-snapshot` | 拍摄文件系统快照 | 目录路径 | JSONL manifest |
| `evo-diff` | 对比两个快照 | 两个 manifest | JSON 变更报告 |
| `evo-monitor` | 包装命令执行 | 目录 + 命令 | 报告 + manifest |
| `evo-fence` | 实时 inotify 监控 | 目录路径 | 事件流 (JSONL) |
| `evo-revert` | 回滚变更 | 目录 + manifest | 回滚结果 |

## 三、数据格式

### 快照 manifest (JSONL)

```json
{"type":"header","ts":"2026-04-13T01:30:00+08:00","path":"/etc","label":"nginx-pre-install","version":"1"}
{"path":"nginx/nginx.conf","sha256":"abc123...","mode":"644","size":2048,"mtime":1712956200}
{"path":"nginx","type":"dir","mode":"755"}
{"type":"footer","files":42,"dirs":8,"total_bytes":102400,"elapsed_ms":120}
```

### 变更报告

```json
{
  "status": "ok",
  "action": "diff",
  "stats": {"created": 3, "deleted": 1, "modified": 2, "permission": 0},
  "changes": [
    {"type": "created", "path": "nginx/conf.d/new-site.conf", "new": {...}},
    {"type": "modified", "path": "nginx/nginx.conf", "old_sha256": "...", "new_sha256": "..."},
    {"type": "deleted", "path": "nginx/old.conf", "old": {...}}
  ]
}
```

### 监控报告

```json
{
  "ts": "...",
  "label": "nginx-upgrade",
  "cmd": "yum install nginx",
  "cmd_exit": 0,
  "diff_stats": {"created": 5, "modified": 12, "deleted": 2, "permission": 0},
  "unexpected_count": 1,
  "unexpected_changes": [{"type": "created", "path": "/etc/suspicious.txt"}],
  "verdict": "WARNING"
}
```

## 四、预期变更清单

通过 `--expected` 参数指定一个 JSONL 文件，列出本次安装**预期会变更的路径**：

```json
{"path": "nginx/nginx.conf"}
{"path": "nginx/conf.d/"}
{"path": "nginx/logs/"}
```

不在清单中的变更会被标记为 `unexpected`，报告的 `verdict` 变为 `WARNING`。

## 五、回滚策略

`evo-revert` 基于 before/after 快照对比：

| 变更类型 | 回滚方式 |
|---------|---------|
| created | 删除新文件 |
| deleted | 从备份恢复目录结构 |
| modified | 从备份恢复文件内容 |
| permission | 恢复原始权限 |

**备份要求**：需要在 before 快照时将原始文件副本存入 `$EVO_HOME/backups/`。
可以集成到 `evo-install` 中，让每次安装自动创建备份。

## 六、实时监控

`evo-fence` 基于 inotify，适合以下场景：
- 监控非 UtopOS 管理的第三方安装过程
- 排查「谁改了我的文件」
- 长期后台监控某个关键目录

```
evo-fence /etc --timeout 300 &
yum install some-package
# 检查 fence 日志
```

## 七、与现有脚本的集成

### evo-install 集成

在 `evo-install` 前后自动调用 snapshot：

```bash
# evo-install 内部
evo-snapshot "$TARGET" --name "${PKG}-pre-install"
# ... 执行安装 ...
evo-snapshot "$TARGET" --name "${PKG}-post-install"
evo-diff before.manifest after.manifest --text
```

### evo-rollback 集成

`evo-rollback` 处理包管理器层面的回滚（yum history undo / nixos-rebuild --rollback），
`evo-revert` 处理非包管理器管理的文件变更（临时文件、权限恢复等）。

两者互补，不冲突。

## 八、依赖

- `sha256sum` (coreutils)
- `find` (findutils)
- `stat` (coreutils)
- `inotifywait` (inotify-tools) — 仅 evo-fence 需要
- `python3` — diff 对比和 JSON 处理

无额外依赖，所有工具在主流 Linux 发行版默认安装。
