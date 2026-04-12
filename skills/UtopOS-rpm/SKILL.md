# UtopOS-rpm — RPM 后端专项

> 本 skill 是 `UtopOS` 的子 skill，专注 RPM 系（Rocky / RHEL / Fedora / CentOS）。
> 通用工作流见父 skill：`UtopOS/SKILL.md`

## 前置

- 系统必须有 `rpm` 命令
- 需要 `yumdownloader` 或 `dnf download --source`
- 需要 `rpmbuild`（在 `rpm-build` 或 `rpmdevtools` 包中）
- 检测命令：`evo-detect`，确认 `backend == "rpm"`

## RPM 构建体系速查（给 Agent 看的）

### 目录结构

```
~/rpmbuild/
├── BUILD/       # 构建过程中的工作目录
├── RPMS/        # 构建产出的 RPM 包
│   └── x86_64/
├── SOURCES/     # 源码 tarball + patch 文件
├── SPECS/       # .spec 文件（构建配方）
└── SRPMS/       # 源码 RPM
```

### Spec 文件结构

```spec
Name:           nginx
Version:        1.24.0
Release:        1%{?dist}
Summary:        A high performance web server
License:        BSD
URL:            https://nginx.org
Source0:        https://nginx.org/download/nginx-%{version}.tar.gz

# 补丁声明
Patch0:         fix-timeout.patch
Patch1:         fix-upstream-buffer.patch

BuildRequires:  gcc, pcre-devel, openssl-devel
Requires:       pcre, openssl

%description
Nginx is a high performance web server...

%prep
%setup -q                        # 解压 Source0
%patch0 -p1                      # 应用 Patch0
%patch1 -p1                      # 应用 Patch1

%build
./configure --prefix=/usr ...
make %{?_smp_mflags}

%install
make DESTDIR=%{buildroot} install

%files
%{_sbindir}/nginx
%{_sysconfdir}/nginx/nginx.conf

%changelog
* Fri Apr 12 2026 UtopOS <evo@local> - 1.24.0-1
- Fix upstream timeout default value
```

### Spec 关键指令

| 指令 | 说明 |
|------|------|
| `%setup -q` | 解压 Source0 到 BUILD/ |
| `%patch0 -p1` | 应用 Patch0（剥掉第一层路径） |
| `%patch0 -p0` | 不剥路径直接应用 |
| `%autosetup -p1` | 自动解压 + 自动应用所有 Patch |
| `%{buildroot}` | 安装目标根目录 |
| `%{_sbindir}` | `/usr/sbin` 宏 |
| `%{?dist}` | 发行版标识，如 `.el9` |

## 源码获取细节

```bash
# evo-fetch-source 内部做的事
# 1. 下载 SRPM
dnf download --source nginx        # 或: yumdownloader --source nginx
# → 得到: nginx-1.24.0-1.el9.src.rpm

# 2. 安装 SRPM（解压到 ~/rpmbuild/）
rpm -ivh nginx-1.24.0-1.el9.src.rpm
# → SOURCES/: nginx-1.24.0.tar.gz + 各种 .patch
# → SPECS/: nginx.spec

# 3. 解压源码
cd ~/rpmbuild/SOURCES/
tar xf nginx-1.24.0.tar.gz -C /tmp/evo-fix-nginx/src/
```

**已安装的包 vs repo 中的包**：
- 已安装：`rpm -qi nginx` 可以直接查询
- 未安装但 repo 中有：`dnf info nginx` 查询
- SRPM 版本可能和二进制 RPM 版本号略有差异（release 号不同）

## 补丁工作流

### evo-build 内部做的事

```bash
# 1. 复制 patch 到 SOURCES
cp /root/.evo/patches/nginx/fix.patch ~/rpmbuild/SOURCES/

# 2. 在 spec 中注册 patch（如果还没注册）
# 检查是否已有 Patch 行，没有则追加
grep -q 'Patch99:' nginx.spec || echo 'Patch99: fix.patch' >> nginx.spec

# 3. 在 %prep 中应用 patch
# 找到 %setup 或 %autosetup 行，在其后添加 %patch99
sed -i '/^%setup/a %patch99 -p1' nginx.spec

# 4. 构建
rpmbuild -ba ~/rpmbuild/SPECS/nginx.spec
# → RPMS/x86_64/nginx-1.24.0-1.el9.x86_64.rpm
# → SRPMS/nginx-1.24.0-1.el9.src.rpm
```

### 手动修改源码（不用 patch 文件）

有时直接 sed 替换比生成 patch 更简单：

```spec
%prep
%setup -q
# 直接修改源码
sed -i 's/default_timeout 60/default_timeout 120/' src/http/ngx_http_core_module.c
```

### 补丁生成技巧

```bash
# 在解压的源码上修改
cd ~/rpmbuild/BUILD/nginx-1.24.0/
cp src/http/ngx_http_core_module.c src/http/ngx_http_core_module.c.orig
vim src/http/ngx_http_core_module.c   # 修改

# 生成标准 patch
diff -u src/http/ngx_http_core_module.c.orig \
        src/http/ngx_http_core_module.c \
        > ~/rpmbuild/SOURCES/fix-timeout.patch
```

**patch 文件命名**：用有意义的名字，数字前缀控制应用顺序：
```
0001-fix-timeout.patch
0002-fix-upstream-buffer.patch
```

## 构建 + 验证

```bash
# 构建 RPM（不安装）
rpmbuild -ba ~/rpmbuild/SPECS/nginx.spec
# -ba = 构建二进制 + 源码 RPM
# -bb = 只构建二进制 RPM
# -bs = 只构建源码 RPM

# 测试安装（不实际执行）
rpm -Uvh --test ~/rpmbuild/RPMS/x86_64/nginx-*.rpm

# 用 UtopOS
scripts/evo-build nginx --patch /root/.evo/patches/nginx/fix.patch
scripts/evo-verify nginx     # → rpm -Uvh --test + 依赖检查
scripts/evo-install nginx    # → dnf/yum localinstall
```

### 常见构建错误

| 错误 | 原因 | 解决 |
|------|------|------|
| `Failed build dependencies` | 缺少 BuildRequires | `dnf builddep nginx.spec` |
| `Patch X does not apply` | patch 格式或路径不对 | 检查 `-p` 参数，检查 patch 基础路径 |
| `File not found: *.spec` | SRPM 没正确安装 | `rpm -ivh *.src.rpm` 重试 |
| `Permission denied` | rpmbuild 目录权限 | `chmod -R 755 ~/rpmbuild` |

## 安装

```bash
# 方式一：yum/dnf localinstall（推荐，处理依赖）
yum localinstall ~/rpmbuild/RPMS/x86_64/nginx-*.rpm
# 或
dnf localinstall ~/rpmbuild/RPMS/x86_64/nginx-*.rpm

# 方式二：rpm 直接安装（不处理依赖）
rpm -Uvh --force ~/rpmbuild/RPMS/x86_64/nginx-*.rpm

# 用 UtopOS
scripts/evo-install nginx
# → 自动选 dnf/yum → 记录事务 ID
```

## 回滚

```bash
# 查看事务历史
yum history list nginx
# → ID    | Command line
# → 15    | localinstall nginx-1.24.0-1.el9.x86_64.rpm
# → 14    | update nginx

# 查看事务详情
yum history info 15

# 回滚到指定事务
yum history undo 15

# 用 UtopOS
scripts/evo-rollback nginx              # → 自动找最近的事务 undo
scripts/evo-rollback nginx --to 15      # → undo 指定事务
```

**回滚限制**：
- 如果事务涉及多个包，undo 会影响所有包
- 某些操作不可 undo（如 `yum remove`）
- 如果旧版本不在 repo 中了，`yum downgrade` 会失败

## 常见 RPM 特有问题

### "rpmbuild 目录不存在"

```bash
# 初始化构建环境
mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
# 或安装 rpmdevtools 自动创建
dnf install -y rpmdevtools
rpmdev-setuptree
```

### "BuildRequires 不满足"

```bash
# 自动安装所有构建依赖
dnf builddep ~/rpmbuild/SPECS/nginx.spec
```

### "想查看源码但不知道在哪"

```bash
# 已解压的源码
ls ~/rpmbuild/BUILD/nginx-*/

# 或从 UtopOS 工作目录
ls /tmp/evo-fix-nginx/src/
```

### "spec 文件太长看不懂"

```bash
# 只看关键部分
grep -E '^(Name|Version|Release|Source|Patch|BuildRequires|%prep|%build|%install)' ~/rpmbuild/SPECS/nginx.spec
```

## 发行版差异

| 发行版 | 包管理 | 源码下载 | 构建依赖 |
|--------|--------|---------|---------|
| RHEL 8/9 | yum / dnf | `dnf download --source` | `dnf builddep` |
| Rocky 8/9 | dnf | `dnf download --source` | `dnf builddep` |
| Fedora | dnf | `dnf download --source` | `dnf builddep` |
| CentOS 7 | yum | `yumdownloader --source` | `yum-builddep` |

**注意**：CentOS 7 的 yum 没有 `builddep` 命令，需要先 `yum install yum-utils`。

## Spec 文件 Patch 编号约定

evo 使用 `Patch99` 作为默认编号，避免和已有 Patch 冲突：

```spec
# 原始 spec 中可能有:
Patch0: upstream-fix-1.patch
Patch1: upstream-fix-2.patch

# evo 追加:
Patch99: evo-fix-timeout.patch

%prep
%autosetup -p1    # 自动应用所有 Patch
# 如果不是 %autosetup，需要手动加:
# %patch0 -p1
# %patch1 -p1
# %patch99 -p1     ← evo 的补丁
```
