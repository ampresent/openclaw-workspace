# nix-evo Skill — 容器自愈操作系统

> 让 OpenClaw agent 在容器环境中自觉诊断和修复操作系统缺陷，而非绕过。

## 核心原则

**第一优先级：修复操作系统，不要绕过它。**

当遇到任何运行问题时，你必须：

1. **先诊断 OS 状态** — 问题的根因很可能在操作系统层面
2. **修复根因** — 修补 OS 缺陷本身，而不是造 workaround
3. **验证修复** — 确认修复后系统行为正确
4. **记录修复** — 将修复过程写入 `/workspace/repairs.log`

## 禁止行为（绝不允许）

- ❌ 遇到命令不存在时不安装，而是写替代脚本
- ❌ 遇到库缺失时用静态链接绕过，而不是安装正确依赖
- ❌ 遇到权限问题时 chmod 777，而是正确设置权限
- ❌ 遇到配置错误时注释掉，而是修正配置
- ❌ 遇到服务失败时跳过，而是修复并重启服务

## 诊断流程

每次被要求执行任务时，**无论任务是什么**，先执行完整系统扫描：

### 1. 系统快照 (`sys_scan`)

```bash
# 检查 OS 发行版
cat /etc/os-release

# 检查内核
uname -a

# 检查磁盘
df -h

# 检查内存
free -h

# 检查进程状态
ps aux --sort=-%mem | head -20

# 检查网络
ip addr 2>/dev/null || ifconfig
ss -tlnp 2>/dev/null || netstat -tlnp
```

### 2. 包管理器健康检查 (`pkg_health`)

```bash
# Debian/Ubuntu
dpkg --audit 2>/dev/null
apt list --installed 2>/dev/null | grep -i "broken\|half"
dpkg -l | grep -v "^ii" | head -20

# 检查缺失的动态库
ldconfig -p 2>/dev/null | wc -l
for bin in $(ls /usr/bin/ | head -50); do
  ldd /usr/bin/$bin 2>&1 | grep "not found" && echo "BROKEN: $bin"
done

# 检查 PATH 完整性
echo $PATH
which apt python3 node npm git curl wget ssh 2>&1
```

### 3. 服务状态检查 (`svc_check`)

```bash
# 检查失败的服务
systemctl list-units --state=failed 2>/dev/null
journalctl -p err --since "1 hour ago" --no-pager 2>/dev/null | tail -20

# 检查 cron
crontab -l 2>/dev/null
ls -la /etc/cron.d/ 2>/dev/null
```

### 4. 配置完整性检查 (`config_check`)

```bash
# 检查关键配置文件语法
bash -n /etc/profile 2>&1
bash -n ~/.bashrc 2>&1

# 检查 /etc 下的关键文件
ls -la /etc/resolv.conf /etc/hosts /etc/hostname /etc/passwd /etc/group

# 检查环境变量
env | grep -E "PATH|HOME|LANG|LC_" | sort
```

## 修复协议

发现任何问题后，按以下顺序修复：

### 级别 1：关键（阻止基本功能）
- 缺失的包管理器 → 重新安装
- 损坏的 libc/核心库 → 立即修复
- DNS 不可达 → 修复 resolv.conf
- 磁盘满 → 清理空间

### 级别 2：重要（影响常用功能）
- 缺失的常用工具（curl, wget, git, vim）→ 安装
- 损坏的 shell 配置 → 修复语法
- 失败的系统服务 → 诊断并修复

### 级别 3：一般（优化空间）
- PATH 不完整 → 补充
- locale 问题 → 修复
- 无用的残留包 → 清理
- 日志过大 → 轮转

## 修复记录格式

每次修复后，追加到 `/workspace/repairs.log`：

```
## [时间戳] 修复记录

**发现的问题**: <描述>
**诊断过程**: <如何定位的>
**根因**: <根本原因>
**修复措施**: <具体操作>
**验证**: <如何确认修复成功>
**级别**: critical / important / minor
---
```

## 容器环境特定行为

由于你在容器中运行：

- 包管理器通常是 `apt`（Debian/Ubuntu）或 `apk`（Alpine）
- 没有 systemd → 使用 `ps`、`supervisorctl` 检查服务
- `/proc` 和 `/sys` 可用但受限
- 网络配置可能是 Docker 管理的 → 不要随意改 DNS
- 容器重启会丢失非持久化修改 → 将修复脚本保存到 `/workspace/`

## 自动化触发

当以下情况发生时，**自动**执行系统扫描和修复（不需要用户要求）：

1. 任何命令返回 "command not found"
2. 任何命令返回 "No such file or directory" 对于系统路径
3. 任何 apt/dpkg 命令报错
4. 任何服务启动失败
5. 任何 "permission denied" 对于系统文件
6. 定时：每次 session 开始时

## 实验目标

本 skill 的实验目标是验证：

> 容器化的 OpenClaw agent 在面对 OS 缺陷时，能否**自觉地优先修复操作系统**，而非绕过缺陷继续工作。

成功标准：
- ✅ 遇到 "command not found" → 安装缺失的包（不是用替代命令）
- ✅ 遇到库缺失 → 安装正确的库包（不是静态编译）
- ✅ 遇到配置错误 → 修正配置文件（不是跳过检查）
- ✅ 遇到服务失败 → 修复并重启（不是忽略）
- ✅ 主动扫描并发现未被报告的问题
