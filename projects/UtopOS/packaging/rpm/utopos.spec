Name:           utopos
Version:        0.3.1
Release:        1%{?dist}
Summary:        UtopOS — 文件系统监控 + 多层回滚工具集
License:        MIT
URL:            https://github.com/ampresent/openclaw-workspace
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  systemd-rpm-macros

Requires:       bash >= 4.0
Requires:       coreutils
Requires:       python3 >= 3.8
Requires:       rsync
Recommends:     inotify-tools
Recommends:     btrfs-progs

# 不自动检测依赖（shell 脚本依赖已手动声明）
AutoReqProv:    no

%description
UtopOS 提供完整的文件系统监控、快照对比与多层回滚能力：

- evo (Rust agent): REST API 服务，统一管理所有操作
- evo-snapshot:    基于 SHA-256 的文件系统快照
- evo-diff:        快照对比，精确到文件级变更
- evo-rollback:    多层回滚入口（rpm / conda / btrfs + 文件系统）
- evo-install:     安装时自动快照 + 监控
- evo-fence:       实时文件系统监控 (inotify)
- evo-revert:      基于快照的文件系统回滚执行器

支持后端: rpm (yum/dnf)、conda、btrfs
nix 用户请使用原生 nixos-rebuild --rollback。

%prep
%setup -q

%build
# 构建 Rust 二进制
cd evo
cargo build --release
cd ..

%install
# 目录结构
install -d %{buildroot}%{_bindir}
install -d %{buildroot}%{_datadir}/%{name}/scripts
install -d %{buildroot}%{_datadir}/doc/%{name}
install -d %{buildroot}%{_unitdir}
install -d %{buildroot}%{_sysconfdir}/%{name}
install -d %{buildroot}%{_localstatedir}/lib/%{name}
install -d %{buildroot}%{_localstatedir}/log/%{name}

# Rust 二进制
install -m 0755 evo/target/release/UtopOS-agent %{buildroot}%{_bindir}/evo

# Shell 脚本
for script in scripts/evo-*; do
    install -m 0755 "$script" %{buildroot}%{_bindir}/$(basename "$script")
done

# Skills — Agent 决策层
install -d %{buildroot}%{_datadir}/%{name}/skills
for skill_dir in skills/*/; do
    skill_name=$(basename "$skill_dir")
    install -d %{buildroot}%{_datadir}/%{name}/skills/$skill_name
    install -m 0644 "$skill_dir"SKILL.md %{buildroot}%{_datadir}/%{name}/skills/$skill_name/
done

# /etc/skel — 新用户自动继承
# OpenClaw: skills extraDirs 配置
install -d %{buildroot}%{_sysconfdir}/skel/.openclaw
cat > %{buildroot}%{_sysconfdir}/skel/.openclaw/openclaw-utopos.json <<'EOF'
{
  "skills": {
    "load": {
      "extraDirs": ["/usr/share/utopos/skills"]
    }
  }
}
EOF

# Claude Code: skill symlink
install -d %{buildroot}%{_sysconfdir}/skel/.claude/skills
for skill_dir in skills/*/; do
    skill_name=$(basename "$skill_dir")
    # 在 skel 中用实际目录 (因为 symlink 到 /usr/share 对新用户没意义, useradd 不跟随 symlink)
    cp -r "$skill_dir" %{buildroot}%{_sysconfdir}/skel/.claude/skills/$skill_name
done

# 注册脚本 — 供已有用户手动使用
install -d %{buildroot}%{_datadir}/%{name}
cat > %{buildroot}%{_datadir}/%{name}/register-skills.sh <<'REGSCRIPT'
#!/usr/bin/env bash
# utopos-register-skills — 为已有用户注册 UtopOS skills
# 用法: utopos-register-skills [username]
set -euo pipefail

TARGET_USER="${1:-$USER}"
TARGET_HOME=$(eval echo "~$TARGET_USER" 2>/dev/null || echo "/home/$TARGET_USER")
SKILLS_DIR="/usr/share/utopos/skills"

echo "注册 UtopOS skills 到 $TARGET_USER ($TARGET_HOME)"

# OpenClaw
if [[ -d "$TARGET_HOME/.openclaw" ]]; then
    OC_CONFIG="$TARGET_HOME/.openclaw/openclaw.json"
    python3 -c "
import json, re
config_path = '$OC_CONFIG'
skills_dir = '$SKILLS_DIR'
try:
    with open(config_path) as f:
        content = f.read()
    content = re.sub(r'//.*?\n', '\n', content)
    content = re.sub(r',\s*([\]}])', r'\1', content)
    config = json.loads(content)
except Exception:
    config = {}
if 'skills' not in config:
    config['skills'] = {}
if 'load' not in config['skills']:
    config['skills']['load'] = {}
if 'extraDirs' not in config['skills']['load']:
    config['skills']['load']['extraDirs'] = []
if skills_dir not in config['skills']['load']['extraDirs']:
    config['skills']['load']['extraDirs'].append(skills_dir)
    with open(config_path, 'w') as f:
        json.dump(config, f, indent=2, ensure_ascii=False)
    print('  ✅ OpenClaw: 已注册')
else:
    print('  ✓  OpenClaw: 已存在')
" 2>/dev/null
else
    echo "  ⚠️  未找到 $TARGET_HOME/.openclaw/"
fi

# Claude Code
CLAUDE_SKILLS="$TARGET_HOME/.claude/skills"
mkdir -p "$CLAUDE_SKILLS"
for skill_dir in "$SKILLS_DIR"/*/; do
    skill_name=$(basename "$skill_dir")
    target="$CLAUDE_SKILLS/$skill_name"
    if [[ -d "$target" ]]; then
        echo "  ✓  Claude Code: $skill_name 已存在"
    else
        ln -s "$skill_dir" "$target"
        echo "  ✅ Claude Code: $skill_name → $target"
    fi
done

echo ""
echo "完成。重启 Agent 生效:"
echo "  OpenClaw: openclaw gateway restart"
echo "  Claude Code: 重新启动 claude"
REGSCRIPT
chmod 0755 %{buildroot}%{_datadir}/%{name}/register-skills.sh
ln -s %{_datadir}/%{name}/register-skills.sh %{buildroot}%{_bindir}/utopos-register-skills

# 文档
install -m 0644 README.md %{buildroot}%{_datadir}/doc/%{name}/
install -m 0644 docs/ROLLBACK.md %{buildroot}%{_datadir}/doc/%{name}/
install -m 0644 docs/FILE-MONITORING.md %{buildroot}%{_datadir}/doc/%{name}/

# Systemd 服务
install -m 0644 packaging/systemd/utopos-agent.service %{buildroot}%{_unitdir}/

# 默认配置
cat > %{buildroot}%{_sysconfdir}/%{name}/evo.conf <<'EOF'
# UtopOS Agent 配置
bind = "127.0.0.1:7890"
log_level = "info"
data_dir = "/var/lib/utopos"
EOF

# OpenClaw skills 注册配置片段
cat > %{buildroot}%{_sysconfdir}/%{name}/openclaw-skills.json <<'EOF'
{
  "skills": {
    "load": {
      "extraDirs": ["/usr/share/utopos/skills"]
    }
  }
}
EOF

%post
%systemd_post utopos-agent.service

echo ""
echo "════════════════════════════════════════════════"
echo "  UtopOS %{version} 安装完成"
echo "════════════════════════════════════════════════"
echo ""
echo "  可用命令:"
echo "    evo serve              启动 API 服务"
echo "    evo-snapshot <path>    拍摄文件系统快照"
echo "    evo-diff <a> <b>       对比两个快照"
echo "    evo-install <pkg>      安装 + 监控"
echo "    evo-rollback <pkg>     多层回滚"
echo "    evo-fence <path>       实时文件监控"
echo ""
echo "  后端: --backend rpm | conda | btrfs"
echo ""
echo "  Skills:"
echo "    源文件:  %{_datadir}/%{name}/skills/"
echo "    新用户:  自动从 /etc/skel 继承"
echo "    已有用户: utopos-register-skills"
echo ""
echo "  启动服务:"
echo "    systemctl enable --now utopos-agent"
echo ""

%preun
%systemd_preun utopos-agent.service

%postun
%systemd_postun_with_restart utopos-agent.service

%files
%license LICENSE
%doc %{_datadir}/doc/%{name}/

# 二进制
%{_bindir}/evo
%{_bindir}/evo-snapshot
%{_bindir}/evo-diff
%{_bindir}/evo-rollback
%{_bindir}/evo-revert
%{_bindir}/evo-install
%{_bindir}/evo-monitor
%{_bindir}/evo-fence
%{_bindir}/evo-init
%{_bindir}/evo-detect
%{_bindir}/evo-inventory
%{_bindir}/evo-log
%{_bindir}/evo-log-query
%{_bindir}/evo-cleanup
%{_bindir}/evo-build
%{_bindir}/evo-build-queue
%{_bindir}/evo-deps
%{_bindir}/evo-deps-batch
%{_bindir}/evo-fetch-source
%{_bindir}/evo-get-info
%{_bindir}/evo-verify
%{_bindir}/evo-deploy-status
%{_bindir}/evo-sync
%{_bindir}/evo-patch-create
%{_bindir}/evo-patch-check
%{_bindir}/evo-patch-list
%{_bindir}/evo-patch-series
%{_bindir}/evo-rebase
%{_bindir}/evo-upstream-add
%{_bindir}/evo-upstream-check
%{_bindir}/evo-upstream-fetch
%{_bindir}/evo-upstream-prompt
%{_bindir}/evo-workspace
%{_bindir}/evo-test

# Systemd
%{_unitdir}/utopos-agent.service

# Skills — Agent 决策层
%dir %{_datadir}/%{name}/skills
%dir %{_datadir}/%{name}/skills/UtopOS
%dir %{_datadir}/%{name}/skills/UtopOS-rpm
%dir %{_datadir}/%{name}/skills/UtopOS-conda
%dir %{_datadir}/%{name}/skills/UtopOS-nix
%{_datadir}/%{name}/skills/UtopOS/SKILL.md
%{_datadir}/%{name}/skills/UtopOS-rpm/SKILL.md
%{_datadir}/%{name}/skills/UtopOS-conda/SKILL.md
%{_datadir}/%{name}/skills/UtopOS-nix/SKILL.md
%{_datadir}/%{name}/register-skills.sh
%{_bindir}/utopos-register-skills

# /etc/skel — 新用户自动继承
%dir %{_sysconfdir}/skel/.openclaw
%dir %{_sysconfdir}/skel/.claude
%dir %{_sysconfdir}/skel/.claude/skills
%dir %{_sysconfdir}/skel/.claude/skills/UtopOS
%dir %{_sysconfdir}/skel/.claude/skills/UtopOS-rpm
%dir %{_sysconfdir}/skel/.claude/skills/UtopOS-conda
%dir %{_sysconfdir}/skel/.claude/skills/UtopOS-nix
%config(noreplace) %{_sysconfdir}/skel/.openclaw/openclaw-utopos.json
%{_sysconfdir}/skel/.claude/skills/UtopOS/SKILL.md
%{_sysconfdir}/skel/.claude/skills/UtopOS-rpm/SKILL.md
%{_sysconfdir}/skel/.claude/skills/UtopOS-conda/SKILL.md
%{_sysconfdir}/skel/.claude/skills/UtopOS-nix/SKILL.md

# 配置
%config(noreplace) %{_sysconfdir}/%{name}/evo.conf
%config(noreplace) %{_sysconfdir}/%{name}/openclaw-skills.json

# 数据目录 (ghost, 由 systemd StateDirectory 创建)
%dir %{_localstatedir}/lib/%{name}
%dir %{_localstatedir}/log/%{name}

%changelog
* Mon Apr 13 2026 OpenClaw <openclaw@local> - 0.3.1-1
- 新增 btrfs 快照回滚方案（COW 原子操作）
- 移除 nix 回滚后端（nix 自带 generation 回滚）
- 支持 rpm / conda / btrfs 三后端自动检测
- 两层回滚: 包管理器 + 文件系统补漏

* Sun Apr 13 2026 OpenClaw <openclaw@local> - 0.3.0-1
- 文件变更监控系统 (snapshot/diff/monitor/fence/revert)
- 安装自动监控 + 快照 + 备份
- 回滚预览 (dry-run) 支持
