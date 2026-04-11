# 状态

- **阶段**: v0.4 实现中
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12 06:09

## 进度

| 模块 | 状态 | 备注 |
|------|------|------|
| 设计文档 | ✅ 完成 | v0.1 + v0.2 + v0.3 设计 |
| README | ✅ 完成 | |
| QUICKSTART | ✅ 完成 | 5 步快速入门指南 |
| CONTRIBUTING | ✅ 完成 | 开发者指南 |
| HOW-TO-CONTRIBUTE | ✅ 完成 | 深度开发者指南 (Round 17) |
| SECURITY | ✅ 完成 | 威胁模型 + 加固计划 |
| INTEGRATIONS | ✅ 完成 | 多主机、Docker、K8s 路线图 |
| API-STABILITY | ✅ 完成 | API 版本管理策略 (Round 18) |
| agent (axum HTTP) | ✅ v0.4 完成 | 39 个端点 |
| MCP Server | ✅ v0.3 完成 | 13 tools + hosts.toml + SSH 隧道 |
| NixOS modules | ✅ 完成 | nextcloud, jellyfin, monitoring-stack (Round 16) |
| 单元测试 | ✅ 完成 | 风险评估 + 包解析 + hosts.toml + MCP 工具路由 |
| AI 配置生成 | ✅ 模板模式 | 9 种 NixOS 模式匹配 |
| 备份系统 | ✅ 完成 | 快照 + 轮转 + 恢复 |
| config_test | ✅ 完成 | test-before-switch + 自动切换 |
| Docker 集成 | ✅ 完成 | 容器发现 + compose 验证 + NixOS 替代建议 (Round 13) |
| CI/CD | ✅ 完成 | Webhook + 预览部署 + 部署追踪 (Round 14) |
| 可观测性 | ✅ 完成 | 结构化日志 + Prometheus 指标 + 告警规则 (Round 15) |
| 开发模式 | ✅ 完成 | 模拟系统 + mock 端点 (Round 17) |
| API 版本管理 | ✅ 完成 | v1/v2 + 弃用头 (Round 18) |
| 智能顾问 | ✅ 完成 | 回滚评分 + 容量规划 (Round 19) |
| TLS | 📋 设计完成 | tls.rs 配置结构，实现待 rustls 集成 |
| SSH 隧道 | ✅ MCP 端完成 | 自动建立 SSH 隧道 |

## API 端点清单 (39)

### 核心 (10)
| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /health | 健康检查 |
| GET | /api/snapshot | 系统快照 |
| GET | /api/logs | 服务日志 |
| GET | /api/config | 读取配置 |
| GET | /api/package | 包信息 |
| GET | /api/generations | Generation 列表/对比 |
| POST | /api/config/validate | Dry-build 验证 |
| POST | /api/config/apply | 应用配置 |
| POST | /api/config/diff | 配置差异 |
| POST | /api/rollback | 回滚 |

### 备份 (4)
| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/backups | 备份列表 |
| POST | /api/backup/create | 创建备份 |
| POST | /api/backup/restore | 恢复备份 |
| POST | /api/backup/rotate | 轮转清理 |

### AI + 测试 (3)
| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/config/generate | AI 配置生成 |
| POST | /api/config/test | Test-before-switch |
| POST | /api/config/test/cancel | 取消自动切换 |

### Docker (2)
| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/docker/status | Docker 环境状态 |
| POST | /api/docker/compose-validate | 验证 compose 文件 |

### CI/CD (4)
| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/cicd/webhook | Git webhook 接收 |
| POST | /api/cicd/preview-deploy | 预览部署 |
| GET | /api/cicd/deployments | 部署历史 |
| GET | /api/cicd/deployments/:id | 部署详情 |

### 可观测性 (6)
| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/observability/logs | 查询结构化日志 |
| GET | /api/observability/metrics | Prometheus 指标 |
| GET | /api/observability/alerts | 告警规则 + 活跃告警 |
| POST | /api/observability/alerts/check | 评估告警条件 |
| POST | /api/observability/alerts/rules | 添加/更新告警规则 |
| GET | /api/observability/config | 集成配置 |

### 开发模式 (6)
| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/dev/mode | 切换开发模式 |
| GET | /api/dev/status | 开发模式状态 |
| POST | /api/dev/mock/service | 设置模拟服务 |
| POST | /api/dev/mock/generation | 模拟配置变更 |
| POST | /api/dev/mock/reset | 重置模拟系统 |
| GET | /api/dev/mock/snapshot | 模拟系统快照 |

### 智能顾问 (2)
| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/advisor/rollback | 回滚推荐 |
| GET | /api/advisor/capacity | 容量分析 |

### API 管理 (1)
| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/versions | API 版本列表 |

## NixOS 模块

| 模块 | 描述 |
|------|------|
| nextcloud.nix | PostgreSQL + Redis + 自动 SSL + 备份 |
| jellyfin.nix | GPU 转码 + Nginx + 媒体扫描 |
| monitoring-stack.nix | Prometheus + Grafana + Loki + Alertmanager |

## 关键决策

- 从 GitOps pull 模型改为 AI 诊断 + 安全执行模型
- 核心价值：NixOS 的人话接口，不是配置同步工具
- Docker 镜像自动映射到 NixOS 原生服务（13 个已知替代）
- 开发模式让非 NixOS 环境也能测试完整 API
- API v1 稳定，v2 beta 中（多主机 + WebSocket）
- 回滚顾问从 feature/experimental 分支 cherry-pick
