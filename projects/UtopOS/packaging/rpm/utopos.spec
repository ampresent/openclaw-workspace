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

SKILLS_DIR="%{_datadir}/%{name}/skills"
OPENCLAW_EXTRA="/usr/share/utopos/skills"

echo ""
echo "════════════════════════════════════════════════"
echo "  UtopOS %{version} 安装完成"
echo "════════════════════════════════════════════════"
echo ""

# ── 1. 注册到 OpenClaw ──
register_openclaw() {
    local oc_home="$1"
    local oc_config="$oc_home/openclaw.json"
    [[ -f "$oc_config" ]] || return 1

    python3 -c "
import json, re
config_path = '$oc_config'
skills_dir = '$OPENCLAW_EXTRA'
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
    print('  ✅ OpenClaw: skills 已注册 → ' + config_path)
else:
    print('  ✓  OpenClaw: skills 已存在')
" 2>/dev/null
}

# ── 2. 注册到 Claude Code ──
register_claude() {
    local claude_home="$1"
    local claude_skills="$claude_home/skills"

    mkdir -p "$claude_skills"

    for skill_dir in "$SKILLS_DIR"/*/; do
        local skill_name=$(basename "$skill_dir")
        local target="$claude_skills/$skill_name"

        if [[ -d "$target" ]]; then
            echo "  ✓  Claude Code: $skill_name 已存在"
        else
            # 创建符号链接到系统安装的 skill
            ln -s "$skill_dir" "$target"
            echo "  ✅ Claude Code: $skill_name → $target"
        fi
    done
}

# ── 自动检测并注册 ──
OPENCLAW_FOUND=false
CLAUDE_FOUND=false

# OpenClaw
for oc_dir in "$HOME/.openclaw" /root/.openclaw /home/*/.openclaw; do
    [[ -d "$oc_dir" ]] || continue
    if register_openclaw "$oc_dir" 2>/dev/null; then
        OPENCLAW_FOUND=true
    fi
done

# Claude Code — 全局
for claude_dir in "$HOME/.claude" /root/.claude /home/*/.claude; do
    [[ -d "$claude_dir" ]] || continue
    register_claude "$claude_dir"
    CLAUDE_FOUND=true
done

# 如果没有找到 ~/.claude，为 root 创建
if [[ "$CLAUDE_FOUND" == "false" ]]; then
    register_claude "/root/.claude"
    CLAUDE_FOUND=true
    echo "  ✅ Claude Code: 已创建 /root/.claude/skills/"
fi

# 如果没有找到 OpenClaw 配置
if [[ "$OPENCLAW_FOUND" == "false" ]]; then
    echo "  ⚠️  未找到 OpenClaw 配置，安装后手动添加:"
    echo '    { "skills": { "load": { "extraDirs": ["/usr/share/utopos/skills"] } } }'
fi

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
echo "  文档: %{_datadir}/doc/%{name}/"
echo ""
echo "  Skills 已注册:"
echo "    - OpenClaw:  ~/.openclaw/openclaw.json → extraDirs"
echo "    - Claude:    ~/.claude/skills/ → symlink"
echo "    - 源文件:    %{_datadir}/%{name}/skills/"
echo ""
echo "  启动服务:"
echo "    systemctl enable --now utopos-agent"
echo ""

%preun
%systemd_preun utopos-agent.service

# 卸载时清理注册
if [[ $1 -eq 0 ]]; then
    # OpenClaw: 移除 extraDirs
    for d in /root/.openclaw /home/*/.openclaw; do
        OC_CONFIG="$d/openclaw.json"
        [[ -f "$OC_CONFIG" ]] || continue
        python3 -c "
import json, re
config_path = '$OC_CONFIG'
skills_dir = '/usr/share/utopos/skills'
try:
    with open(config_path) as f:
        content = f.read()
    content = re.sub(r'//.*?\n', '\n', content)
    content = re.sub(r',\s*([\]}])', r'\1', content)
    config = json.loads(content)
    dirs = config.get('skills', {}).get('load', {}).get('extraDirs', [])
    if skills_dir in dirs:
        dirs.remove(skills_dir)
        with open(config_path, 'w') as f:
            json.dump(config, f, indent=2, ensure_ascii=False)
        print('  ✅ OpenClaw: 已注销 skills')
except Exception:
    pass
" 2>/dev/null
    done

    # Claude Code: 移除 symlink
    for d in /root/.claude /home/*/.claude; do
        CLAUDE_SKILLS="$d/skills"
        [[ -d "$CLAUDE_SKILLS" ]] || continue
        for skill_name in UtopOS UtopOS-rpm UtopOS-conda UtopOS-nix; do
            link="$CLAUDE_SKILLS/$skill_name"
            if [[ -L "$link" && "$(readlink "$link")" == "/usr/share/utopos/skills/$skill_name" ]]; then
                rm -f "$link"
                echo "  ✅ Claude Code: 已移除 $skill_name"
            fi
        done
    done
fi

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
