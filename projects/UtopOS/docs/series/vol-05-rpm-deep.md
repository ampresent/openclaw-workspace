# 第五卷：RPM 后端完全指南

---

## 5.1 RPM 构建体系

### 目录约定

```bash
~/rpmbuild/
├── BUILD/       # 构建过程中的工作目录（解压、编译在这里）
├── BUILDROOT/   # 安装暂存目录（make install DESTDIR=...）
├── RPMS/        # 构建产出的二进制 RPM
│   ├── x86_64/
│   └── noarch/
├── SOURCES/     # 源码 tarball + 补丁文件
├── SPECS/       # .spec 文件（构建配方）
└── SRPMS/       # 源码 RPM
```

初始化：
```bash
mkdir -p ~/rpmbuild/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
# 或
dnf install -y rpmdevtools && rpmdev-setuptree
```

---

## 5.2 Spec 文件完全解析

```spec
# ====== 包元数据 ======
Name:           nginx                          # 包名
Version:        1.24.0                         # 上游版本
Release:        1%{?dist}                      # 打包版本（%?dist = .el9）
Summary:        A high performance web server  # 简短描述
License:        BSD-2-Clause                   # 许可证
URL:            https://nginx.org              # 项目主页
Source0:        https://nginx.org/download/nginx-%{version}.tar.gz
# 源码 URL，%{version} 自动展开为 1.24.0

# ====== 补丁声明 ======
Patch0:         fix-timeout.patch              # 补丁文件名（对应 SOURCES/ 下的文件）
Patch1:         fix-upstream-buffer.patch
Patch99:        evo-custom-fix.patch           # UtopOS 使用 Patch99 避免冲突

# ====== 依赖 ======
BuildRequires:  gcc make                       # 构建时依赖
BuildRequires:  pcre2-devel openssl-devel
Requires:       pcre2 openssl                  # 运行时依赖
Requires(pre):  shadow-utils                   # 安装前需要

# ====== 描述 ======
%description
Nginx is a high performance HTTP and reverse proxy server.

# ====== 构建流程 ======
%prep                                         # 准备阶段
%setup -q                                     # 解压 Source0，-q 安静模式
%patch0 -p1                                   # 应用 Patch0（剥掉第一层目录）
%patch1 -p1
%patch99 -p1                                  # 应用 UtopOS 补丁

%build                                        # 构建阶段
./configure \
    --prefix=%{_prefix} \
    --with-http_ssl_module \
    %{?_with_debug:--with-debug}
make %{?_smp_mflags}                          # 并行编译

%install                                      # 安装到 BUILDROOT
make DESTDIR=%{buildroot} install

%files                                        # 声明包中包含的文件
%{_sbindir}/nginx
%config(noreplace) %{_sysconfdir}/nginx/nginx.conf
%{_mandir}/man8/nginx.8.gz

%changelog                                    # 变更日志
* Fri Apr 12 2026 UtopOS <evo@local> - 1.24.0-1
- Fix upstream timeout default value
```

---

## 5.3 Spec 关键指令

### %prep 阶段

| 指令 | 说明 |
|------|------|
| `%setup` | 解压 Source0 到 BUILD/ |
| `%setup -q` | 安静模式 |
| `%setup -c` | 创建目录后解压（不剥掉顶层目录） |
| `%setup -n myname` | 指定解压后的目录名 |
| `%patch0 -p1` | 应用 Patch0 |
| `%patch0 -p0` | 不剥路径直接应用 |
| `%autosetup -p1` | 自动解压 + 自动应用所有 Patch |

### 宏定义

| 宏 | 展开为 |
|----|--------|
| `%{_prefix}` | `/usr` |
| `%{_sbindir}` | `/usr/sbin` |
| `%{_sysconfdir}` | `/etc` |
| `%{_datadir}` | `/usr/share` |
| `%{_libdir}` | `/usr/lib64` |
| `%{_mandir}` | `/usr/share/man` |
| `%{_smp_mflags}` | `-j$(nproc)` |
| `%{buildroot}` | 安装暂存目录 |
| `%{?dist}` | 发行版标识（`.el9`、`.fc39`） |
| `%{?_with_debug}` | `--with-debug` 时展开 |

### %install 安装

```spec
%install
make DESTDIR=%{buildroot} install

# 手动安装
install -D -m 644 nginx.conf %{buildroot}%{_sysconfdir}/nginx/nginx.conf
install -D -m 755 nginx %{buildroot}%{_sbindir}/nginx

# 创建目录
mkdir -p %{buildroot}%{_datadir}/nginx/html
```

### %files 文件列表

```spec
%files
# 二进制
%{_sbindir}/nginx

# 配置文件（noreplace = 更新时不覆盖用户修改）
%config(noreplace) %{_sysconfdir}/nginx/nginx.conf

# 目录
%dir %{_sysconfdir}/nginx/
%dir %{_datadir}/nginx/

# 文档
%doc README CHANGES LICENSE
%license LICENSE

# 所有文件（慎用）
%{_datadir}/nginx/
```

---

## 5.4 补丁工作流

### UtopOS 的补丁方式

```bash
# 1. Agent 修改源码
cd /tmp/evo-fix-nginx/src/
vim src/http/ngx_http_core_module.c

# 2. evo-patch-create 生成补丁
scripts/evo-patch-create nginx --desc "fix timeout"

# 3. evo-build 自动处理：
#    - 复制 patch → ~/rpmbuild/SOURCES/
#    - 在 spec 中注册 Patch99
#    - 在 %prep 中添加 %patch99
#    - 执行 rpmbuild -ba
```

### 补丁命名约定

- 补丁文件用有意义的名字
- UtopOS 使用 `Patch99` 避免与现有 Patch 编号冲突
- 如果 `Patch99` 已被占用，按需改为 `Patch98`、`Patch97` 等

### 补丁生成

```bash
# 正确的补丁格式
cd ~/rpmbuild/BUILD/nginx-1.24.0/
cp src/http/ngx_http_core_module.c src/http/ngx_http_core_module.c.orig
vim src/http/ngx_http_core_module.c

diff -u src/http/ngx_http_core_module.c.orig \
        src/http/ngx_http_core_module.c \
        > ~/rpmbuild/SOURCES/fix-timeout.patch
```

补丁格式：
```diff
--- a/src/http/ngx_http_core_module.c.orig
+++ b/src/http/ngx_http_core_module.c
@@ -123,7 +123,7 @@
-    default_timeout = 60;
+    default_timeout = 120;
```

---

## 5.5 rpmbuild 用法

```bash
# 构建二进制 + 源码 RPM
rpmbuild -ba ~/rpmbuild/SPECS/nginx.spec

# 只构建二进制
rpmbuild -bb ~/rpmbuild/SPECS/nginx.spec

# 只构建源码
rpmbuild -bs ~/rpmbuild/SPECS/nginx.spec

# 定义宏
rpmbuild -ba --define "with_debug 1" nginx.spec
```

### 常见错误

| 错误 | 原因 | 解决 |
|------|------|------|
| `Failed build dependencies` | 缺少 BuildRequires | `dnf builddep nginx.spec` |
| `Patch X does not apply` | patch 路径不对 | 检查 -p 参数 |
| `File listed twice` | files 列表重复 | 去重 |
| `Installed (but unpackaged) file(s)` | 文件未声明 | 加到 %files |
| `RPM build errors` | 通用错误 | 看上方具体原因 |

---

## 5.6 安装与回滚

### 安装

```bash
# 推荐：dnf/yum localinstall（处理依赖）
dnf localinstall ~/rpmbuild/RPMS/x86_64/nginx-*.rpm

# 直接安装（不处理依赖）
rpm -Uvh --force ~/rpmbuild/RPMS/x86_64/nginx-*.rpm
```

### 事务历史

```bash
yum history list nginx
# ID | Command line
# 15 | localinstall nginx-1.24.0-1.el9.x86_64.rpm
# 14 | update nginx

yum history info 15     # 详情
yum history undo 15     # 回滚
```

---

## 5.7 发行版差异

| 特性 | RHEL 8/9 | Rocky 8/9 | Fedora 39+ | CentOS 7 |
|------|----------|-----------|------------|----------|
| 包管理 | dnf | dnf | dnf | yum |
| 源码下载 | `dnf download --source` | 同左 | 同左 | `yumdownloader --source` |
| 构建依赖 | `dnf builddep` | 同左 | 同左 | `yum-builddep` |
| Python | 3.6/3.9 | 同左 | 3.12 | 2.7 |
| 默认编译器 | GCC 8/11 | 同左 | GCC 13 | GCC 4.8 |
