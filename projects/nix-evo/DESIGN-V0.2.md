# nix-evo v0.2 设计

> 下一阶段功能设计文档

## 1. SSH 隧道自动建立

### 目标

MCP Server 自动为远程主机建立 SSH 隧道，无需用户手动操作。

### 设计

在 `hosts.toml` 中配置 SSH 隧道：

```toml
[hosts.production]
url = "http://127.0.0.1:7890"
ssh_tunnel = "admin@prod-server:7890"
token = "prod-token"
```

MCP Server 启动时或首次连接时：

1. 检查 `ssh_tunnel` 字段
2. 如果隧道未建立，自动执行 `ssh -L 7890:127.0.0.1:7890 -N -f admin@prod-server`
3. 等待隧道就绪（检测本地端口）
4. 通过隧道访问 agent API

### 实现要点

- 隧道进程管理：记录 PID，启动时检查、退出时清理
- 超时处理：SSH 连接 10 秒超时
- 重连逻辑：连接断开时自动重试
- 依赖：系统需要 `ssh` 命令

### Agent 端

无需改动。Agent 始终只监听 127.0.0.1，SSH 隧道是 MCP 端职责。

## 2. nixos-rebuild test 后再 switch

### 目标

`config_apply` 改为两步：先 `test`（不写 bootloader，可重启恢复），观察一段时间后再 `switch`。

### 设计

新增 `config_test` 端点：

```
POST /api/config/test
{
  "config": "...",
  "message": "测试配置变更",
  "auto_switch_after": 300  // 秒，默认 5 分钟
}
```

流程：

1. 写入临时配置
2. 执行 `nixos-rebuild test`（不修改 bootloader，重启后自动恢复）
3. 等待 `auto_switch_after` 秒
4. 如果期间没有收到取消请求，自动执行 `nixos-rebuild switch`
5. 如果收到取消请求或超时后失败，保持 test 状态（重启即恢复）

### MCP 端工具

新增 `config_test` 工具：

```json
{
  "name": "config_test",
  "description": "先测试再切换，重启后自动恢复（如果未 switch）",
  "parameters": {
    "host": "string",
    "config": "NixOS 配置内容",
    "message": "变更说明",
    "wait_seconds": 300
  }
}
```

### 风险评估升级

- `config_test` 的风险评估阈值降低（可容忍更高风险，因为 test 可逆）
- `config_apply` 增加检查：如果最近有 test 但没 switch，提示先确认 test 结果

## 3. Secrets 管理集成

### 目标

集成 agenix 或 sops-nix，让 AI agent 能安全地管理 secrets（密码、证书、API key）。

### 设计

#### 方案 A: agenix 集成（推荐）

agenix 使用 age 加密，secrets 存储在 git repo 中：

```
secrets/
  db-password.age    # 加密的 secret
  secrets.nix        # secret 定义
```

Agent 端新增端点：

```
GET  /api/secrets          → 列出 secrets（不显示值）
POST /api/secrets/set     → 设置/更新 secret
POST /api/secrets/rotate  → 生成新密钥
```

MCP 端工具：

- `secrets_list` — 列出所有 secrets 及其部署状态
- `secrets_set` — 设置 secret 值（加密后写入）
- `secrets_rotate` — 为指定 secret 生成新的随机值

#### 安全约束

- Agent 端**不存储** secrets 的明文
- `secrets_set` 接收明文 → 使用 recipient 公钥加密 → 写入 .age 文件
- `secrets_list` 只返回 metadata（文件名、修改时间、部署目标）
- MCP 端可以显示 "已配置 N 个 secrets" 而不是值

#### 实现步骤

1. 检测 `secrets/` 目录是否存在（agenix 格式）
2. 检测 `sops.yaml` 是否存在（sops-nix 格式）
3. 根据检测结果选择后端
4. 实现 CRUD 操作

### 依赖

- Agent 需要 `rage` 或 `age` 命令（加密/解密）
- 用户需要配置 recipient 公钥

## 4. 实现优先级

| 功能 | 优先级 | 复杂度 | 依赖 |
|------|--------|--------|------|
| SSH 隧道自动建立 | 高 | 低 | 无 |
| nixos-rebuild test | 中 | 中 | 无 |
| secrets 管理 | 低 | 高 | agenix/sops-nix |
