# 第七卷：安全与信任模型

---

## 7.1 安全威胁模型

### 威胁分类

| 威胁 | 风险 | 缓解措施 |
|------|------|---------|
| 补丁引入安全漏洞 | 高 | 构建前审计 + 风险分级 |
| 补丁破坏系统功能 | 中 | dry-run 验证 + 回滚机制 |
| 未授权变更 | 中 | 变更历史记录 (JSONL) |
| 补丁覆盖用户配置 | 低 | %config(noreplace) + overlay |
| 重复构建浪费资源 | 低 | 缓存机制 + 清理脚本 |

### 信任边界

```
┌─── 信任 ─────────────────────────────────────────────┐
│  用户                                               │
│  - 确认关键决策                                     │
│  - 配置 trust.toml                                  │
│  - 审计变更历史                                     │
├─────────────────────────────────────────────────────│
│  AI Agent                                           │
│  - 遵循 SKILL.md 约束                               │
│  - 生成风险评估                                     │
│  - 展示摘要（不展示 raw diff 给非技术用户）         │
├─────────────────────────────────────────────────────│
│  Scripts (evo-*)                                    │
│  - 执行机械操作                                     │
│  - 不做决策                                         │
│  - 输出 JSON 供 Agent 消费                          │
├─────────────────────────────────────────────────────│
│  包管理器 (nix/rpm/conda)                           │
│  - 管理包的安装和回滚                               │
│  - 提供事务性保证                                   │
└─────────────────────────────────────────────────────┘
```

---

## 7.2 风险分级

### 三级分类

| 等级 | 判定标准 | 示例 | 确认策略 |
|------|---------|------|---------|
| 🟢 safe | 只改配置默认值，不删包，不影响核心功能 | 改 nginx worker_connections 默认值 | 可配置自动 |
| 🟡 moderate | 源码逻辑修改，影响单一功能，有回滚方案 | 改 upstream timeout 计算逻辑 | 必须确认 |
| 🔴 dangerous | 核心模块修改，影响面广，回滚代价高 | 改内存分配器、改 TLS 握手 | 必须确认 + 二次确认 |

### 自动分级规则

`evo-patch-create` 内部分级逻辑：

```
if 只改配置模板/默认值 → safe
elif 改了核心模块 (main.c, ssl, crypto, 内存管理) → dangerous
elif 改了 ≥3 个文件 或 ≥100 行 → moderate
elif 改了 <3 个文件 且 <100 行 → moderate
else → moderate (默认)
```

Agent 可以在 `--risk` 参数中手动覆盖分级。

---

## 7.3 信任白名单

### 配置文件：`~/.evo/trust.toml`

```toml
[trust.nginx]
safe_auto = true
risk_levels = ["safe"]

[trust.php]
safe_auto = true
risk_levels = ["safe", "moderate"]

[trust.openssl]
safe_auto = false
# 即使 safe 也不自动，因为 openssl 影响面广
```

### 决策逻辑

```
1. patch.risk == "dangerous"
   → 永远不自动 apply，无论 trust.toml 怎么配

2. 读 ~/.evo/trust.toml，找 [trust.<pkg>]

3. 找到配置？
   ├─ patch.risk ∈ risk_levels → 跳过确认
   └─ patch.risk ∉ risk_levels → 正常确认

4. 没找到配置？ → 正常确认
```

### 安全限制

- `dangerous` 级永远不自动，即使在白名单中
- 白名单变更需要用户手动编辑 trust.toml
- Agent 不自动修改 trust.toml
- 建议只对熟悉的包开启白名单

---

## 7.4 变更审计

### 变更历史

所有安装操作记录到 `~/.evo/history/<pkg>.jsonl`：

```jsonl
{"ts":"2026-04-12T16:00:00+08:00","pkg":"nginx","backend":"rpm","action":"install","txn_id":"15"}
{"ts":"2026-04-12T16:05:00+08:00","pkg":"nginx","backend":"rpm","action":"rollback","txn_id":"15"}
```

### 补丁元数据

每个补丁附带 `.meta.json`：

```json
{
  "pkg": "nginx",
  "desc": "修复 upstream timeout 默认值",
  "ticket": "ISSUE-123",
  "risk": "moderate",
  "created": "2026-04-12T16:00:00+08:00",
  "patch_file": "~/.evo/patches/nginx/fix-timeout.patch",
  "size_bytes": 1024,
  "files_changed": 1,
  "insertions": 5,
  "deletions": 2
}
```

### 审计查询

```bash
# 查看某包的变更历史
cat ~/.evo/history/nginx.jsonl | python3 -m json.tool

# 查看所有补丁
find ~/.evo/patches/ -name "*.meta.json" -exec cat {} \;

# 查看构建日志
ls -la ~/.evo/logs/
```

---

## 7.5 反模式清单

| 编号 | 反模式 | 原因 | 正确做法 |
|------|--------|------|---------|
| AM-01 | `vim /etc/nginx/nginx.conf` | 下次更新覆盖 | 修源码 → 打包 → 安装 |
| AM-02 | `systemctl restart nginx` | 不解决根因 | 找根因 → 修源码 |
| AM-03 | 跳过 `evo-verify` | 可能安装破坏性变更 | 永远先 verify |
| AM-04 | `pip install` in conda | 破坏 conda 环境一致性 | 只用 conda |
| AM-05 | 不记录就修改 | 无法追溯 | `evo-patch-create` + git |
| AM-06 | 改 /nix/store | store 是只读的，且重建时覆盖 | overlay + overrideAttrs |
| AM-07 | `rpm -Uvh --force` 不验证 | 可能覆盖关键文件 | 先 `rpm -Uvh --test` |
| AM-08 | 修改后不递增 build number | conda 认为是旧版 | 递增 build number |

---

## 7.6 回滚策略

### 每个后端的回滚机制

| 后端 | 回滚方式 | 粒度 | 代价 |
|------|---------|------|------|
| Nix | generation 切换 | 整个系统配置 | 极低（只是切指针） |
| RPM | yum history undo | 单个事务（可多包） | 低 |
| Conda | install --revision | 整个环境 | 中 |

### 回滚时机

```bash
# 安装后验证
scripts/evo-install nginx
systemctl status nginx

# 如果服务异常
scripts/evo-rollback nginx
# 或直接用包管理器回滚
```

### 回滚限制

- **Nix**：如果 GC 已清理旧 generation，无法回滚到那个版本
- **RPM**：如果旧版本不在 repo 中，`yum downgrade` 会失败
- **Conda**：revision 回滚是环境级别的，不只是单个包

---

## 7.7 安全最佳实践

1. **始终验证** — 不跳过 `evo-verify`
2. **谨慎配置白名单** — 只对熟悉的包开启 safe_auto
3. **审计变更历史** — 定期检查 `~/.evo/history/`
4. **保留构建日志** — `~/.evo/logs/` 有完整的构建输出
5. **不要分享 ~/.evo** — 补丁和配置可能包含敏感信息
6. **生产环境先测后装** — 在测试环境验证后再在生产安装
