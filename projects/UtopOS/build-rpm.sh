#!/usr/bin/env bash
# build-rpm.sh — 一键构建 UtopOS RPM 包
# 用法: ./build-rpm.sh [--install] [--local]
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SPEC="$PROJECT_ROOT/packaging/rpm/utopos.spec"
VERSION=$(grep '^Version:' "$SPEC" | awk '{print $2}')
RELEASE=$(grep '^Release:' "$SPEC" | awk '{print $2}' | sed 's/%{?dist}//')
NAME="utopos"

DO_INSTALL=false
LOCAL_BUILD=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install|-i) DO_INSTALL=true; shift ;;
        --local|-l)   LOCAL_BUILD=true; shift ;;
        -h|--help)
            echo "用法: $0 [--install] [--local]"
            echo "  --install  构建后自动安装 RPM"
            echo "  --local    使用本地 ~/rpmbuild 目录 (不依赖 mock)"
            exit 0
            ;;
        *) shift ;;
    esac
done

echo "════════════════════════════════════════════"
echo "  构建 $NAME $VERSION-$RELEASE"
echo "════════════════════════════════════════════"

# 1. 安装构建依赖
echo ""
echo "▸ 检查构建依赖..."
MISSING=()
for cmd in rpmbuild cargo rustc gcc; do
    command -v "$cmd" &>/dev/null || MISSING+=("$cmd")
done

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  缺少: ${MISSING[*]}"
    echo "  尝试安装..."
    if command -v dnf &>/dev/null; then
        sudo dnf install -y rpm-build rpmdevtools rust cargo gcc 2>/dev/null || true
    elif command -v yum &>/dev/null; then
        sudo yum install -y rpm-build rpmdevtools rust cargo gcc 2>/dev/null || true
    fi
fi

# 2. 准备 rpmbuild 目录
echo "▸ 准备 rpmbuild 目录结构..."
if [[ "$LOCAL_BUILD" == "true" ]]; then
    RPMBUILD="$HOME/rpmbuild"
    mkdir -p "$RPMBUILD"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
else
    RPMBUILD=$(rpm --eval '%{_topdir}' 2>/dev/null || echo "$HOME/rpmbuild")
    mkdir -p "$RPMBUILD"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
fi

# 3. 打源码包
echo "▸ 打包源码 tarball..."
TARBALL="$RPMBUILD/SOURCES/$NAME-$VERSION.tar.gz"
tar czf "$TARBALL" \
    --transform "s,^,$NAME-$VERSION/," \
    --exclude='.git' \
    --exclude='target' \
    --exclude='node_modules' \
    --exclude='*.tar.gz' \
    -C "$PROJECT_ROOT" \
    evo scripts skills docs packaging README.md CONTRIBUTING.md

echo "  → $TARBALL ($(du -h "$TARBALL" | cut -f1))"

# 4. 复制 spec
cp "$SPEC" "$RPMBUILD/SPECS/"

# 5. 构建 RPM
echo ""
echo "▸ 构建 RPM..."
rpmbuild -ba "$RPMBUILD/SPECS/utopos.spec" \
    --define "_topdir $RPMBUILD" \
    2>&1 | tail -20

# 6. 输出结果
echo ""
RPM_FILE=$(find "$RPMBUILD/RPMS" -name "$NAME-$VERSION-*.rpm" -type f | head -1)
SRC_FILE=$(find "$RPMBUILD/SRPMS" -name "$NAME-$VERSION-*.src.rpm" -type f | head -1)

if [[ -n "$RPM_FILE" ]]; then
    echo "════════════════════════════════════════════"
    echo "  ✅ 构建成功"
    echo "════════════════════════════════════════════"
    echo "  RPM: $RPM_FILE"
    [[ -n "$SRC_FILE" ]] && echo "  SRPM: $SRC_FILE"
    echo "  大小: $(du -h "$RPM_FILE" | cut -f1)"
    echo ""

    if [[ "$DO_INSTALL" == "true" ]]; then
        echo "▸ 安装中..."
        sudo rpm -Uvh --force "$RPM_FILE"
        echo "  ✅ 已安装"
        echo ""
        echo "  启动服务: systemctl enable --now utopos-agent"
    else
        echo "  安装: sudo rpm -ivh $RPM_FILE"
    fi
else
    echo "❌ 构建失败，请检查上方日志"
    exit 1
fi
