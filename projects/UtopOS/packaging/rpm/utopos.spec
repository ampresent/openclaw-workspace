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
echo "  文档: %{_datadir}/doc/%{name}/"
echo ""
echo "  Skills (Agent 决策层):"
echo "    %{_datadir}/%{name}/skills/"
echo "    - UtopOS      通用操作手册"
echo "    - UtopOS-rpm  RPM 后端专用"
echo "    - UtopOS-conda Conda 后端专用"
echo "    - UtopOS-nix  Nix 后端参考"
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

# 配置
%config(noreplace) %{_sysconfdir}/%{name}/evo.conf

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
