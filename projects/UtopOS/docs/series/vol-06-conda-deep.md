# 第六卷：Conda 后端完全指南

---

## 6.1 Conda 构建体系

### 核心概念

- **Package**: 一个可安装的 tarball，包含编译好的文件 + 元数据
- **Recipe**: 构建配方（meta.yaml + build.sh + 其他文件）
- **Feedstock**: conda-forge 社区的标准包仓库
- **Channel**: 包的仓库（conda-forge、defaults、私有 channel）
- **Build number**: 包的构建编号，每次修改递增

### Feedstock 结构

```
numpy-feedstock/
├── recipe/
│   ├── meta.yaml       # 包元数据
│   ├── build.sh        # Linux/macOS 构建脚本
│   ├── bld.bat         # Windows 构建脚本
│   ├── run_test.sh     # 测试脚本
│   ├── 0001-fix.patch  # 补丁文件
│   └── conda_build_config.yaml  # 构建矩阵配置
├── .ci_support/        # CI 构建配置
│   ├── linux_64_.yaml
│   └── osx_64_.yaml
├── .scripts/           # CI 脚本
└── README.md
```

---

## 6.2 meta.yaml 完全解析

```yaml
# ====== 变量定义（Jinja2） ======
{% set name = "mypackage" %}
{% set version = "1.2.3" %}

# ====== 包信息 ======
package:
  name: {{ name|lower }}
  version: {{ version }}

# ====== 源码 ======
source:
  url: https://pypi.io/packages/source/{{ name[0] }}/{{ name }}/{{ name }}-{{ version }}.tar.gz
  sha256: abc123def456...
  patches:
    - fix-buffer-overflow.patch    # ← UtopOS 补丁加在这里
    - fix-listen-port.patch

# ====== 构建 ======
build:
  number: 0                        # ← 每次修改必须递增
  script: {{ PYTHON }} -m pip install . -vv  # 默认调用 build.sh
  skip: true  # [py<38]            # 跳过 Python < 3.8
  noarch: python                   # 纯 Python 包，不需要编译

# ====== 依赖 ======
requirements:
  build:                           # 构建时需要（已废弃，用 host）
    - {{ compiler('c') }}          # C 编译器
    - make
  host:                            # 构建时需要的库
    - python
    - pip
    - setuptools
    - numpy
  run:                             # 运行时需要
    - python >=3.8
    - numpy >=1.20
    - scipy

# ====== 测试 ======
test:
  imports:                         # 导入测试
    - mypackage
    - mypackage.submodule
  commands:                        # 命令测试
    - mypackage --version
  requires:                        # 测试依赖
    - pytest

# ====== 关于 ======
about:
  home: https://github.com/example/mypackage
  license: MIT
  license_file: LICENSE
  summary: A brief description
  description: |
    A longer description that can span
    multiple lines.
  doc_url: https://mypackage.readthedocs.io
  dev_url: https://github.com/example/mypackage

# ====== 额外信息 ======
extra:
  recipe-maintainers:
    - maintainer1
    - maintainer2
```

---

## 6.3 Jinja2 模板语法

```yaml
# 变量定义
{% set version = "1.2.3" %}

# 变量引用
version: {{ version }}

# 字符串过滤
name: {{ name|lower }}       # 小写
name: {{ name|replace("-", "_") }}  # 替换

# 条件
build:
  skip: true  # [win]        # 跳过 Windows
  skip: true  # [py<38]      # 跳过 Python < 3.8
  skip: true  # [not linux]  # 只在 Linux 构建

# 编译器宏
requirements:
  host:
    - {{ compiler('c') }}     # gcc / clang / msvc
    - {{ compiler('cxx') }}   # g++ / clang++ / msvc
```

### 常用选择器

| 选择器 | 含义 |
|--------|------|
| `[linux]` | 只在 Linux |
| `[osx]` | 只在 macOS |
| `[win]` | 只在 Windows |
| `[py>=38]` | Python >= 3.8 |
| `[py<38]` | Python < 3.8 |
| `[np>=120]` | NumPy >= 1.20 |
| `[not linux]` | 非 Linux |
| `[linux and py>=310]` | Linux 且 Python >= 3.10 |

---

## 6.4 build.sh 构建脚本

```bash
#!/bin/bash
set -euo pipefail

# 环境变量（conda-build 自动设置）
# $PREFIX    — 安装前缀（如 /path/to/miniconda3/envs/_build）
# $RECIPE_DIR — recipe 目录
# $SRC_DIR   — 源码目录
# $PYTHON    — Python 解释器路径
# $CC        — C 编译器
# $CXX       — C++ 编译器

# 手动应用补丁（如果没在 meta.yaml 的 patches 中声明）
# patch -p1 < $RECIPE_DIR/fix-buffer.patch

# 标准 Python 包
$PYTHON -m pip install . --no-deps --ignore-installed -vv

# 或 CMake 包
mkdir build && cd build
cmake .. -DCMAKE_INSTALL_PREFIX=$PREFIX
make -j${CPU_COUNT}
make install

# 或 Meson 包
meson setup builddir --prefix=$PREFIX
ninja -C builddir
ninja -C builddir install
```

---

## 6.5 补丁工作流

### UtopOS 的方式

```bash
# 1. Agent 修改源码
cd /tmp/evo-fix-mypackage/src/
vim src/mypackage/core.py

# 2. evo-patch-create 生成补丁
scripts/evo-patch-create mypackage --desc "fix buffer overflow"

# 3. evo-build 自动处理：
#    - 复制 patch → recipe/
#    - 更新 meta.yaml 的 patches 列表
#    - 递增 build number
#    - 执行 conda build
```

### meta.yaml 修改示例

修改前：
```yaml
source:
  url: https://pypi.io/.../mypackage-1.2.3.tar.gz
  sha256: abc123...
build:
  number: 0
```

修改后：
```yaml
source:
  url: https://pypi.io/.../mypackage-1.2.3.tar.gz
  sha256: abc123...
  patches:
    - fix-buffer-overflow.patch
build:
  number: 1
```

---

## 6.6 conda build 用法

```bash
# 构建
conda build recipe/

# 指定 Python 版本
conda build recipe/ --python=3.11

# 指定输出路径
conda build recipe/ --output

# 构建 + 测试
conda build recipe/ --test

# 在新环境构建（推荐）
conda build recipe/ --no-test
```

### 构建输出

```
# conda build recipe/ 输出
/.../miniconda3/conda-bld/linux-64/mypackage-1.2.3-py311_1.tar.bz2
```

---

## 6.7 安装与回滚

### 安装本地构建的包

```bash
conda install --use-local mypackage

# 强制重装
conda install --use-local --force-reinstall mypackage
```

### Revision 回滚

```bash
conda list --revisions
# 2026-04-12 16:00:00  rev 15  (+mypackage-1.2.3)
# 2026-04-11 14:30:00  rev 14  (+numpy-1.26)

conda install --revision 15
```

---

## 6.8 私有 Channel

```bash
# 创建本地 channel
mkdir -p /opt/my-channel/linux-64
cp /path/to/conda-bld/linux-64/*.tar.bz2 /opt/my-channel/linux-64/
conda index /opt/my-channel

# 使用
conda install -c file:///opt/my-channel mypackage

# 或添加为永久 channel
conda config --add channels file:///opt/my-channel
```
