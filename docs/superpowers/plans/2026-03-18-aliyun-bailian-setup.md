# 阿里云百炼平台接入实施计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 OpenClaw 中配置阿里云百炼平台作为模型 provider，接入 DeepSeek 或 MiniMax 模型

**Architecture:** 
- 在 `~/.openclaw/openclaw.json` 和 `~/.openclaw/agents/main/agent/models.json` 中添加阿里云百炼 (bailian) provider 配置
- 配置 API Key 和 base URL
- 添加 DeepSeek 和 MiniMax 模型定义
- 更新默认模型配置

**Tech Stack:** 
- OpenClaw 配置系统 (JSON)
- 阿里云百炼 API (OpenAI 兼容格式)
- DeepSeek-V3 / MiniMax 模型

---

## 前置准备（需要用户手动操作）

### Task 0: 开通阿里云百炼服务并获取 API Key

**说明：** 此步骤需要用户在阿里云官网手动操作，无法自动化

- [ ] **步骤 1: 登录阿里云控制台**

访问：https://home.console.aliyun.com/

- [ ] **步骤 2: 开通百炼服务**

访问：https://bailian.console.aliyun.com/
点击"开通服务"（可能需要实名认证）

- [ ] **步骤 3: 创建 API Key**

1. 访问：https://bailian.console.aliyun.com/api-key
2. 点击"创建新的 API Key"
3. 复制 API Key 并保存（格式如：`sk-xxxxxxxxxxxxxxxx`）

- [ ] **步骤 4: 确认模型可用性**

在百炼控制台确认以下模型可用：
- `deepseek-chat` (DeepSeek-V3 非思考模式)
- `deepseek-reasoner` (DeepSeek-V3 思考模式，可选)
- `miniimax-text-01` (或最新 MiniMax 文本模型)

---

## 配置实施

### Task 1: 更新主配置文件 `~/.openclaw/openclaw.json`

**Files:**
- Modify: `~/.openclaw/openclaw.json`

- [ ] **步骤 1: 在 `models.providers` 中添加/更新 `bailian` provider**

找到现有的 `bailian` provider 配置，更新为：

```json
{
  "models": {
    "providers": {
      "bailian": {
        "api": "openai-completions",
        "apiKey": "sk-YOUR_ACTUAL_API_KEY_HERE",
        "baseUrl": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "models": [
          {
            "id": "deepseek-chat",
            "name": "DeepSeek-V3",
            "reasoning": false,
            "input": ["text"],
            "cost": {
              "input": 0.002,
              "output": 0.008,
              "cacheRead": 0,
              "cacheWrite": 0
            },
            "contextWindow": 128000,
            "maxTokens": 8192
          },
          {
            "id": "deepseek-reasoner",
            "name": "DeepSeek-V3 Reasoner",
            "reasoning": true,
            "input": ["text"],
            "cost": {
              "input": 0.002,
              "output": 0.008,
              "cacheRead": 0,
              "cacheWrite": 0
            },
            "contextWindow": 128000,
            "maxTokens": 8192
          },
          {
            "id": "miniimax-text-01",
            "name": "MiniMax Text 01",
            "reasoning": false,
            "input": ["text"],
            "cost": {
              "input": 0.001,
              "output": 0.003,
              "cacheRead": 0,
              "cacheWrite": 0
            },
            "contextWindow": 128000,
            "maxTokens": 8192
          }
        ]
      },
      "gateway": {
        "api": "openai-completions",
        "apiKey": "eyJhbGciOiJIUzI1NiIsImtpZCI6Ind1eWluZy1rZXktMSJ9...",
        "baseUrl": "https://wyaigw-sales-jvs.wuyinggw.com/v1",
        "models": [
          {
            "contextWindow": 128000,
            "cost": {
              "cacheRead": 0,
              "cacheWrite": 0,
              "input": 0,
              "output": 0
            },
            "id": "qwen3.5-plus",
            "input": ["text"],
            "maxTokens": 4096,
            "name": "qwen3.5-plus",
            "reasoning": false
          }
        ]
      }
    }
  }
}
```

**注意：** 
- 将 `apiKey` 替换为用户在 Task 0 中获取的实际 API Key
- 保留现有的 `gateway` provider 配置

- [ ] **步骤 2: 更新默认模型配置**

在 `agents.defaults.model` 中设置默认使用百炼的 DeepSeek 模型：

```json
{
  "agents": {
    "defaults": {
      "model": {
        "primary": "bailian/deepseek-chat"
      },
      "models": {
        "bailian/deepseek-chat": {},
        "bailian/miniimax-text-01": {},
        "gateway/qwen3.5-plus": {}
      }
    }
  }
}
```

- [ ] **步骤 3: 保存并验证 JSON 格式**

```bash
cd ~/.openclaw && cat openclaw.json | python3 -m json.tool > /dev/null && echo "JSON 格式正确" || echo "JSON 格式错误"
```

预期输出：`JSON 格式正确`

- [ ] **步骤 4: 备份原配置文件**

```bash
cp ~/.openclaw/openclaw.json ~/.openclaw/openclaw.json.bak.$(date +%Y%m%d%H%M%S)
```

---

### Task 2: 更新 Agent 模型配置 `~/.openclaw/agents/main/agent/models.json`

**Files:**
- Modify: `~/.openclaw/agents/main/agent/models.json`

- [ ] **步骤 1: 更新 `bailian` provider 配置**

```json
{
  "providers": {
    "bailian": {
      "baseUrl": "https://dashscope.aliyuncs.com/compatible-mode/v1",
      "apiKey": "sk-YOUR_ACTUAL_API_KEY_HERE",
      "api": "openai-completions",
      "models": [
        {
          "id": "deepseek-chat",
          "name": "DeepSeek-V3",
          "reasoning": false,
          "input": ["text"],
          "cost": {
            "input": 0.002,
            "output": 0.008,
            "cacheRead": 0,
            "cacheWrite": 0
          },
          "contextWindow": 128000,
          "maxTokens": 8192,
          "api": "openai-completions"
        },
        {
          "id": "deepseek-reasoner",
          "name": "DeepSeek-V3 Reasoner",
          "reasoning": true,
          "input": ["text"],
          "cost": {
            "input": 0.002,
            "output": 0.008,
            "cacheRead": 0,
            "cacheWrite": 0
          },
          "contextWindow": 128000,
          "maxTokens": 8192,
          "api": "openai-completions"
        },
        {
          "id": "miniimax-text-01",
          "name": "MiniMax Text 01",
          "reasoning": false,
          "input": ["text"],
          "cost": {
            "input": 0.001,
            "output": 0.003,
            "cacheRead": 0,
            "cacheWrite": 0
          },
          "contextWindow": 128000,
          "maxTokens": 8192,
          "api": "openai-completions"
        }
      ]
    },
    "gateway": {
      "baseUrl": "https://wyaigw-sales-jvs.wuyinggw.com/v1",
      "apiKey": "eyJhbGciOiJIUzI1NiIsImtpZCI6Ind1eWluZy1rZXktMSJ9...",
      "api": "openai-completions",
      "models": [
        {
          "id": "qwen3.5-plus",
          "name": "qwen3.5-plus",
          "reasoning": false,
          "input": ["text"],
          "cost": {
            "input": 0,
            "output": 0,
            "cacheRead": 0,
            "cacheWrite": 0
          },
          "contextWindow": 128000,
          "maxTokens": 4096,
          "api": "openai-completions"
        }
      ]
    }
  }
}
```

- [ ] **步骤 2: 验证 JSON 格式**

```bash
cd ~/.openclaw/agents/main/agent && cat models.json | python3 -m json.tool > /dev/null && echo "JSON 格式正确" || echo "JSON 格式错误"
```

预期输出：`JSON 格式正确`

---

### Task 3: 重启 OpenClaw Gateway 并验证

- [ ] **步骤 1: 重启 Gateway**

```bash
openclaw gateway restart
```

- [ ] **步骤 2: 检查 Gateway 状态**

```bash
openclaw gateway status
```

预期输出：Gateway 运行正常

- [ ] **步骤 3: 测试模型调用**

发送一条测试消息到 OpenClaw，确认模型切换成功：

```
测试：请确认你正在使用哪个模型？
```

- [ ] **步骤 4: 验证响应**

检查响应是否来自 DeepSeek 模型（可通过响应风格或日志确认）

---

### Task 4: 成本监控配置（可选）

- [ ] **步骤 1: 在阿里云百炼控制台设置预算告警**

访问：https://bailian.console.aliyun.com/billing/alert
设置月度预算告警（建议 ¥40）

- [ ] **步骤 2: 记录初始配置**

在 `memory/YYYY-MM-DD.md` 中记录：

```markdown
## 阿里云百炼配置

- 配置日期：2026-03-18
- Provider: bailian (阿里云百炼)
- 默认模型：deepseek-chat
- API Key 位置：~/.openclaw/openclaw.json
- 预算上限：¥40/月
- 监控告警：已设置
```

---

## 验证清单

- [ ] API Key 已正确配置
- [ ] base URL 指向阿里云百炼
- [ ] DeepSeek 模型已添加
- [ ] MiniMax 模型已添加（可选）
- [ ] Gateway 重启成功
- [ ] 模型调用测试通过
- [ ] 预算告警已设置

---

## 故障排查

### 问题 1: API Key 无效
**症状：** 调用模型时返回 401 错误
**解决：** 检查 API Key 是否正确复制，确认百炼服务已开通

### 问题 2: 模型不可用
**症状：** 返回模型不存在错误
**解决：** 在百炼控制台确认模型已启用，检查模型 ID 拼写

### 问题 3: Gateway 无法启动
**症状：** `openclaw gateway restart` 失败
**解决：** 检查 JSON 配置文件格式，查看 `~/.openclaw/logs/` 中的错误日志

---

## 完成后清理

- [ ] 删除计划文件或移至 `archive/`
- [ ] 提交配置变更（如使用 git）
