# nix-evo Skill

> 源码修复工作流 — AI Agent 通过分析和修补源代码来解决系统软件问题。
> 支持 **Nix**、**RPM**、**Conda** 三种包管理后端。

## 核心原则

**永远不要绕过问题，永远不要直接修改操作系统。**

当你（AI Agent）在服务器上遇到系统软件问题时：

1. **❌ 不要**尝试用 workaround 绕过（比如重启服务、改运行时配置糊弄过去）
2. **❌ 不要**直接替换二进制、手动编辑运行时文件、打热补丁
3. **✅ 要**解压源代码，分析源码中的根因
4. **✅ 要**修改源代码，生成补丁，通过包管理器重新打包
5. **✅ 要**通过包管理器安装新包来应用变更，而不是直接动系统文件

## 为什么

直接修改操作系统文件：

- 被下一次系统更新/包升级覆盖
- 无法追溯（谁改了什么、为什么改）
- 无法回滚
- 破坏可复现性

修补源代码 + 重新打包：

- 变更持久化（通过包管理器安装，有记录）
- 变更可追溯（补丁文件 + 版本控制）
- 变更可回滚（包管理器支持降级/回滚）
- 变更可复现（同一构建流程产生相同结果）

## 后端检测

操作前先确定目标系统使用哪种包管理器：

```bash
# 检测
which nixos-rebuild 2>/dev/null && echo "NixOS"
which rpm 2>/dev/null && echo "RPM"
which conda 2>/dev/null && echo "Conda"
```

根据检测结果选择对应的工作流分支。

---

## 通用工作流（六步）

### 第一步：诊断（不要急着修）

使用 `system_snapshot` 和 `service_logs` 了解问题。

```
用户: nginx 502 了

→ 调用 system_snapshot
→ 发现 php-fpm.service failed
→ 调用 service_logs("phpfpm.service")
→ 发现错误信息
```

**关键**：此时不要动手修。先理解根因。

### 第二步：定位源码包

根据后端找到出问题的软件对应的源码包。

### 第三步：解压并分析源代码

**这是核心步骤。** 不要看运行时文件，要看源码。

分析时关注：

- 配置文件模板（`*.conf.in`、`*.service` 模板）
- 默认值是否有问题
- 编译参数是否导致了错误行为
- 源码中的 bug

### 第四步：生成补丁

基于源码分析结果，针对不同后端生成对应格式的补丁。

### 第五步：验证 + 应用

通过包管理器验证（dry-run）再安装，不要直接改文件。

### 第六步：提交到上游（可选但推荐）

如果修复的是上游 bug，向对应的上游仓库提交 PR。

---

## 后端一：Nix / NixOS

### 定位源码包

```bash
# 查找包 derivation
nix-instantiate '<nixpkgs>' -A php83

# 获取源码路径
nix-build '<nixpkgs>' -A php83.src --no-out-link

# 或直接查看 store
ls /nix/store/*-php-*/
```

### 解压源码

```bash
# nix-build 产出的源码目录可直接使用
SRC=$(nix-build '<nixpkgs>' -A php83.src --no-out-link)
cp -r $SRC /tmp/fix-workspace/
```

### 生成补丁

```nix
# 方式一：overlay + overrideAttrs
final: prev: {
  php83 = prev.php83.overrideAttrs (old: {
    patches = (old.patches or []) ++ [
      ./patches/fix-fpm-listen-port.patch
    ];
  });
}
```

```nix
# 方式二：修改 NixOS 模块配置覆盖默认值
services.phpfpm.pools.main.settings = {
  "listen" = "/run/phpfpm.sock";
};
```

### 验证 + 应用

```bash
nixos-rebuild dry-build      # 先验证
nixos-rebuild switch          # 确认后应用
```

通过 nix-evo：

```
→ config_validate（dry-run + 风险评估）
→ 用户确认
→ config_apply（nixos-rebuild switch）
→ 生成新 generation
```

### 回滚

```bash
nixos-rebuild switch --rollback    # 回退到上一个 generation
nixos-rebuild switch --to 42       # 回退到指定 generation
```

### MCP Tools

| 工具 | 用途 |
|------|------|
| `system_snapshot` | 全局状态 |
| `service_logs` | 服务日志 |
| `package_info` | 包信息 |
| `config_read` | 读 NixOS 配置 |
| `config_validate` | dry-run 验证 + 风险评估 |
| `config_apply` | 执行变更 |
| `rollback_list` / `rollback_apply` | 兜底回滚 |

---

## 后端二：RPM（Rocky / RHEL / Fedora）

### 定位源码包

```bash
# 查找提供某文件的包
rpm -qf /usr/sbin/nginx

# 查找已安装包的源码 RPM
rpm -qi nginx | grep "Source RPM"
rpm -q --qf '%{SOURCERPM}\n' nginx

# 下载 SRPM
yumdownloader --source nginx
# 或
dnf download --source nginx
```

### 解压源码

```bash
# 安装 SRPM（解包到 ~/rpmbuild/）
rpm -ivh nginx-*.src.rpm

# 源码目录结构
ls ~/rpmbuild/SOURCES/    # 源码 tarball + 补丁文件
ls ~/rpmbuild/SPECS/      # spec 文件

# 解压源码
cd ~/rpmbuild/SOURCES/
tar xf nginx-*.tar.gz
```

### 分析源码

```bash
# 先看 spec 文件了解构建流程
cat ~/rpmbuild/SPECS/nginx.spec

# 再看源码
cd ~/rpmbuild/SOURCES/nginx-*/
grep -r "listen" --include="*.conf" .
```

### 生成补丁

```bash
# 1. 在解压的源码上修改
cd ~/rpmbuild/SOURCES/nginx-*/
vim src/http/ngx_http_core_module.c   # 修复 bug

# 2. 生成补丁文件
cd ~/rpmbuild/SOURCES/
diff -u nginx-*/src/http/ngx_http_core_module.c.orig \
        nginx-*/src/http/ngx_http_core_module.c \
        > fix-nginx-upstream-timeout.patch

# 3. 在 spec 文件中注册补丁
# 编辑 ~/rpmbuild/SPECS/nginx.spec，添加：
# Patch99: fix-nginx-upstream-timeout.patch
# 在 %prep 的 %setup 后添加：
# %patch99 -p1
```

### 验证 + 应用

```bash
# 构建测试（不安装）
rpmbuild -ba ~/rpmbuild/SPECS/nginx.spec

# 安装新构建的 RPM
rpm -Uvh ~/rpmbuild/RPMS/x86_64/nginx-*.rpm --force
# 或通过 yum/dnf
yum localinstall ~/rpmbuild/RPMS/x86_64/nginx-*.rpm
```

### 回滚

```bash
# 查看历史
yum history list nginx
yum history info <ID>

# 回滚到指定事务
yum history undo <ID>

# 或直接降级
yum downgrade nginx-<旧版本>
```

---

## 后端三：Conda

### 定位源码包

```bash
# 查看已安装包信息
conda list <package>
conda info <package>

# 查找包的 recipe（源码）
conda skeleton pypi <package>           # PyPI 包
# 或从 conda-forge 获取 recipe
git clone https://github.com/conda-forge/<package>-feedstock.git
```

### 解压源码

```bash
# 方式一：从 feedstock 获取
cd <package>-feedstock
ls recipe/              # meta.yaml + build.sh + bld.bat

# 方式二：从已安装的包提取
conda install <package>
# 包安装在 $CONDA_PREFIX/lib/ 或 $CONDA_PREFIX/bin/
```

### 分析源码

```bash
# 先看 recipe 元数据
cat recipe/meta.yaml    # 包名、版本、依赖、构建脚本

# 再看构建脚本
cat recipe/build.sh     # Linux 构建步骤

# 看源码（在 source/ 或手动下载）
```

### 生成补丁

```bash
# 1. 在源码上修改
vim <source_file>

# 2. 生成补丁
diff -u <file>.orig <file> > fix-<desc>.patch

# 3. 补丁放入 recipe 目录
cp fix-<desc>.patch recipe/

# 4. 更新 meta.yaml
# 添加：
# source:
#   patches:
#     - fix-<desc>.patch
```

```yaml
# meta.yaml 示例
package:
  name: mypackage
  version: "1.2.3"

source:
  url: https://example.com/mypackage-1.2.3.tar.gz
  patches:
    - fix-listen-port.patch

build:
  number: 1    # 递增 build number

requirements:
  build:
    - {{ compiler('c') }}
  host:
    - python
```

### 验证 + 应用

```bash
# 本地构建测试
conda build recipe/

# 安装本地构建的包
conda install --use-local mypackage

# 或发布到私有 channel
anaconda upload <path-to-conda-pkg>
```

### 回滚

```bash
# 查看历史
conda list --revisions

# 回滚到指定 revision
conda install --revision <N>

# 或降级到旧版本
conda install mypackage=<old_version>
```

---

## 反模式（绝对不要做）

### ❌ 直接编辑运行时文件

```bash
# NixOS — 禁止
vim /nix/store/abc-php-8.3/etc/php-fpm.conf   # store 是只读的，且会被覆盖
vim /etc/php-fpm.d/www.conf                     # nixos-rebuild 会覆盖

# RPM — 禁止
vim /etc/nginx/nginx.conf                       # yum update 会覆盖
sed -i 's/listen 80/listen 8080/' /usr/sbin/nginx  # 改二进制？不要

# Conda — 禁止
vim $CONDA_PREFIX/lib/python3.11/site-packages/xxx.py  # conda update 会覆盖
```

### ❌ 绕过问题

```bash
# 禁止（任何后端都禁止）
systemctl restart nginx                 # 重启不解决根因
while true; do restart; done            # 更不要这样
```

### ❌ 不经过验证直接安装

```bash
# Nix — 禁止跳过 dry-build
nixos-rebuild switch                    # 没有先 dry-build

# RPM — 禁止跳过测试
rpm -Uvh *.rpm                          # 没有先 rpmbuild 测试

# Conda — 禁止跳过 build test
conda install --use-local pkg           # 没有先 conda build 验证
```

### ❌ 混用包管理器

```bash
# 禁止
pip install numpy                       # 在 conda 环境中用 pip
apt install nginx                       # 在 RPM 系统上用 apt
yum install python-pkg                  # 在 NixOS 上用 yum
```

## 安全约束

- 所有变更先验证（dry-run / test build）再安装
- 风险等级评估：safe / moderate / dangerous
- 补丁文件保存在版本控制中，可追溯
- API 默认只监听 127.0.0.1
- 每个后端支持回滚机制
