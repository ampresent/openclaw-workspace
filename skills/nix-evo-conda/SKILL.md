# nix-evo-conda — Conda 后端专项

> 本 skill 是 `nix-evo` 的子 skill，专注 Conda 包管理。
> 通用工作流见父 skill：`nix-evo/SKILL.md`

## 前置

- 系统必须有 `conda` 命令（Miniconda / Anaconda / Miniforge）
- 需要 `conda-build`：`conda install -n base conda-build`
- 需要 `git`（克隆 feedstock 用）
- 检测命令：`evo-detect`，确认 `backend == "conda"`

## Conda 构建体系速查（给 Agent 看的）

### Feedstock 结构

conda-forge 社区的标准包结构：

```
numpy-feedstock/
├── recipe/
│   ├── meta.yaml      # 包元数据（名字、版本、依赖、源码 URL）
│   ├── build.sh       # Linux/macOS 构建脚本
│   ├── bld.bat        # Windows 构建脚本
│   ├── run_test.sh    # 测试脚本
│   └── *.patch        # 补丁文件
├── .ci_support/       # CI 配置
├── README.md
└── LICENSE
```

### meta.yaml 结构

```yaml
package:
  name: mypackage
  version: "1.2.3"

source:
  url: https://pypi.io/packages/source/m/mypackage/mypackage-{{ version }}.tar.gz
  sha256: abc123...
  patches:
    - fix-listen-port.patch       # ← evo 补丁加在这里
    - fix-buffer-overflow.patch

build:
  number: 0                       # ← 每次修改递增

requirements:
  host:
    - python
    - pip
    - setuptools
  run:
    - python >=3.8
    - numpy

test:
  imports:
    - mypackage

about:
  home: https://github.com/example/mypackage
  license: MIT
  summary: My package description
```

### meta.yaml 关键字段

| 字段 | 说明 |
|------|------|
| `package.name` | 包名 |
| `package.version` | 版本号 |
| `source.url` | 源码下载地址 |
| `source.patches` | 补丁文件列表 |
| `build.number` | 构建编号，每次修改必须递增 |
| `build.script` | 自定义构建命令（默认调用 build.sh） |
| `requirements.host` | 构建时依赖 |
| `requirements.run` | 运行时依赖 |
| `test.imports` | 构建后测试导入 |

### Jinja2 模板变量

meta.yaml 支持 Jinja2 语法：

```yaml
{% set name = "mypackage" %}
{% set version = "1.2.3" %}

package:
  name: {{ name|lower }}
  version: {{ version }}

source:
  url: https://pypi.io/packages/source/{{ name[0] }}/{{ name }}/{{ name }}-{{ version }}.tar.gz

build:
  number: 0
  skip: true  # [win]    # 跳过 Windows
  skip: true  # [py<38]  # 跳过 Python < 3.8
```

## 源码获取细节

```bash
# evo-fetch-source 内部做的事（两种方式）

# 方式一：克隆 conda-forge feedstock（推荐）
git clone https://github.com/conda-forge/mypackage-feedstock.git /tmp/evo-fix-mypackage/src
# → src/recipe/meta.yaml
# → src/recipe/build.sh

# 方式二：conda skeleton（feedstock 不存在时）
conda skeleton pypi mypackage --output-dir /tmp/evo-fix-mypackage/src
# → src/mypackage/meta.yaml
# → src/mypackage/bld.bat, build.sh
```

**选择哪种**：
- conda-forge 有 feedstock → 方式一（更标准，有现成 recipe）
- 私有包或 PyPI 独有 → 方式二（自动生成 recipe）

## 补丁工作流

### evo-build 内部做的事

```bash
# 1. 复制 patch 到 recipe 目录
cp /root/.evo/patches/mypackage/fix.patch /tmp/evo-fix-mypackage/src/recipe/

# 2. 更新 meta.yaml 的 patches 列表
# 如果已有 patches: 段，追加
# 如果没有，在 source: 下添加

# 3. 递增 build number
# sed -i 's/number: 0/number: 1/' recipe/meta.yaml

# 4. 构建
conda build recipe/
# → 输出路径: /path/to/miniconda3/conda-bld/linux-64/mypackage-1.2.3-py311_1.tar.bz2
```

### meta.yaml 补丁修改示例

修改前：
```yaml
source:
  url: https://pypi.io/packages/source/m/mypackage/mypackage-1.2.3.tar.gz
  sha256: abc123...
build:
  number: 0
```

修改后：
```yaml
source:
  url: https://pypi.io/packages/source/m/mypackage/mypackage-1.2.3.tar.gz
  sha256: abc123...
  patches:
    - fix-buffer-overflow.patch      # ← 新增
build:
  number: 1                           # ← 递增
```

### build.sh 补丁应用

如果不想通过 meta.yaml 的 patches 字段，可以在 build.sh 中手动打补丁：

```bash
#!/bin/bash
# recipe/build.sh

# 手动应用补丁
patch -p1 < $RECIPE_DIR/fix-buffer-overflow.patch

# 正常构建
$PYTHON setup.py install --single-version-externally-managed --record=record.txt
```

## 构建 + 验证

```bash
# 构建
conda build recipe/
# 输出: /path/to/conda-bld/linux-64/mypackage-1.2.3-py311_1.tar.bz2

# 测试安装（dry-run）
conda install --dry-run --use-local mypackage

# 用 nix-evo
scripts/evo-build mypackage --patch /root/.evo/patches/mypackage/fix.patch
scripts/evo-verify mypackage    # → conda install --dry-run --use-local
scripts/evo-install mypackage   # → conda install --use-local
```

### 常见构建错误

| 错误 | 原因 | 解决 |
|------|------|------|
| `UnsatisfiableError` | 依赖冲突 | 创建新的 conda 环境构建 |
| `patch does not apply` | patch 路径或版本不对 | 检查 `-p` 参数 |
| `No space left on device` | conda-bld 缓存太大 | `conda clean --all` |
| `recipe/meta.yaml not found` | recipe 路径不对 | 检查 feedstock 结构 |

### 构建环境隔离

```bash
# 推荐：在独立环境构建
conda create -n build-env python=3.11
conda activate build-env
conda build recipe/

# 或指定 Python 版本
conda build recipe/ --python=3.11
```

## 安装

```bash
# 安装本地构建的包
conda install --use-local mypackage

# 强制重新安装
conda install --use-local --force-reinstall mypackage

# 用 nix-evo
scripts/evo-install mypackage
# → conda install --use-local → 记录 revision
```

## 回滚

```bash
# 查看 revision 历史
conda list --revisions
# → 2026-04-12 16:00:00  rev 15
# → 2026-04-11 14:30:00  rev 14

# 查看某个 revision 的详情
conda list --revisions | grep "rev 15"

# 回滚到指定 revision
conda install --revision 15

# 用 nix-evo
scripts/evo-rollback mypackage              # → 回滚到上一个 revision
scripts/evo-rollback mypackage --to 15      # → 回滚到指定 revision
```

**回滚限制**：
- revision 记录的是整个环境的变化，不只是单个包
- 如果旧版本不在 cache 中，需要重新下载
- 跨 channel 的回滚可能失败

## 常见 Conda 特有问题

### "conda build 找不到 recipe"

```bash
# 确认路径
ls /tmp/evo-fix-mypackage/src/recipe/meta.yaml
# 或
ls /tmp/evo-fix-mypackage/src/mypackage/meta.yaml

# 指定 recipe 路径
conda build /tmp/evo-fix-mypackage/src/recipe/
```

### "build number 没递增导致安装失败"

conda 用 build number 判断新旧。如果 patch 了但没递增 build number，`conda install --use-local` 可能认为已经是最新版：

```bash
# 检查当前 build number
grep 'number:' recipe/meta.yaml
# 必须比已安装的版本高

# 强制安装
conda install --use-local --force-reinstall mypackage
```

### "conda clean 释放空间"

```bash
# 清理所有缓存
conda clean --all

# 只清理 tarball
conda clean --tarballs

# 只清理索引
conda clean --index-cache
```

### "环境依赖冲突"

```bash
# 创建干净环境测试
conda create -n test-mypackage python=3.11 mypackage --use-local
conda activate test-mypackage

# 检查依赖
conda list mypackage
```

## 私有 Channel 发布

构建完成后可以发布到私有 channel，其他机器通过 `conda install -c <channel>` 安装：

```bash
# 方式一：anaconda upload
anaconda upload /path/to/conda-bld/linux-64/mypackage-1.2.3-py311_1.tar.bz2

# 方式二：本地文件 channel
mkdir -p /opt/my-channel/linux-64
cp /path/to/conda-bld/linux-64/mypackage-*.tar.bz2 /opt/my-channel/linux-64/
conda index /opt/my-channel

# 使用
conda install -c file:///opt/my-channel mypackage
```

## Conda vs Mamba vs Micromamba

| 工具 | 速度 | 兼容性 | 说明 |
|------|------|--------|------|
| conda | 慢 | 完全 | 标准工具 |
| mamba | 快 | 高 | C++ 重写的 solver |
| micromamba | 最快 | 高 | 单二进制，不需要 base env |

evo 脚本优先用 `conda`，如果检测到 `mamba` 或 `micromamba` 也可以用（未来版本会自动切换）。
