# ─────────────────────────────────────────────────────────────
# UtopOS RPM Builder — CentOS Stream 10
# 用法:
#   docker build -t utopos-builder .
#   docker create --name rpm-out utopos-builder
#   docker cp rpm-out:/root/rpmbuild/RPMS ./rpms
#   docker cp rpm-out:/root/rpmbuild/SRPMS ./srpms
#   docker rm rpm-out
# ─────────────────────────────────────────────────────────────
FROM quay.io/centos/centos:stream10

ARG RUST_VERSION=stable

# ── 系统依赖 ────────────────────────────────────────────────
RUN dnf install -y \
        gcc \
        make \
        rpm-build \
        rpmdevtools \
        systemd-rpm-macros \
        pkg-config \
        systemd-devel \
        python3 \
        rsync \
        shadow-utils \
        curl \
        ca-certificates \
    && dnf clean all

# ── 安装 Rust (rustup) ─────────────────────────────────────
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain ${RUST_VERSION} --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# ── 准备 rpmbuild 目录 ──────────────────────────────────────
RUN rpmdev-setuptree

# ── 复制项目源码 ────────────────────────────────────────────
WORKDIR /build
COPY . /build/UtopOS/

# ── 打源码 tarball ──────────────────────────────────────────
ARG NAME=utopos
ARG VERSION=0.3.1

RUN tar czf ~/rpmbuild/SOURCES/${NAME}-${VERSION}.tar.gz \
    --transform "s,^UtopOS,${NAME}-${VERSION}," \
    --exclude='.git' \
    --exclude='target' \
    --exclude='node_modules' \
    --exclude='container-experiment' \
    --exclude='Dockerfile' \
    -C /build \
    UtopOS

# ── 复制 spec ───────────────────────────────────────────────
RUN cp /build/UtopOS/packaging/rpm/utopos.spec ~/rpmbuild/SPECS/

# ── 预热 Cargo 依赖 ─────────────────────────────────────────
WORKDIR /build/UtopOS/evo
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs \
    && cargo fetch 2>/dev/null || true \
    && rm -rf src

# ── 构建 RPM ────────────────────────────────────────────────
WORKDIR /build
RUN rpmbuild -ba ~/rpmbuild/SPECS/utopos.spec \
        --define "_topdir /root/rpmbuild" \
    && echo "═══════════════════════════════════════" \
    && echo "  RPM 构建完成" \
    && echo "═══════════════════════════════════════" \
    && find ~/rpmbuild/RPMS -name "*.rpm" -exec ls -lh {} \; \
    && find ~/rpmbuild/SRPMS -name "*.rpm" -exec ls -lh {} \;
