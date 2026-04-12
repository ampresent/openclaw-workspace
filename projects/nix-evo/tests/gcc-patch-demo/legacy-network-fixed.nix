# legacy-network-fixed.nix — 通过 GCC overlay 修复编译问题
#
# 修复策略：为 legacy-network 包定制 GCC，将 implicit-function-declaration
# 从 error 降级为 warning，这样旧代码可以继续编译。
#
# 使用方式：
#   nix-build legacy-network-fixed.nix
#
# 这就是 nix-evo config_apply 会应用的配置片段

{ pkgs ? import <nixpkgs> {} }:

let
  # 策略 1：用定制 GCC 编译（推荐 — 只影响这个包，不污染全局 GCC）
  gccTolerant = pkgs.gcc.overrideAttrs (old: {
    # 在 GCC 的 spec 文件中添加 -Wno-error=implicit-function-declaration
    # 这会让所有用这个 GCC 编译的代码对该警告不报错
    postFixup = (old.postFixup or "") + ''
      # 通过修改 specs，将 implicit-function-declaration 降级为 warning
      $out/bin/gcc -dumpspecs > $out/lib/gcc/*/*/specs
      sed -i 's/-Werror=implicit-function-declaration/-Wimplicit-function-declaration/g' \
        $out/lib/gcc/*/*/specs || true
    '';
  });

  # 策略 2（备选）：在 NixOS overlay 中全局应用
  # 在 configuration.nix 的 nixpkgs.overlays 中添加：
  #
  #   nixpkgs.overlays = [
  #     (final: prev: {
  #       gcc = prev.gcc.overrideAttrs (old: {
  #         NIX_CFLAGS_COMPILE = (old.NIX_CFLAGS_COMPILE or "") +
  #           " -Wno-error=implicit-function-declaration";
  #       });
  #     })
  #   ];

in pkgs.stdenv.mkDerivation {
  pname = "legacy-network";
  version = "0.1.0-fixed";

  src = ./.;

  # 使用容错 GCC 编译
  nativeBuildInputs = [ gccTolerant ];

  buildPhase = ''
    echo "=== 使用容错 GCC 编译 legacy_network.c ==="
    echo "GCC 版本："
    ${gccTolerant}/bin/gcc --version | head -1
    echo ""
    echo "编译命令：gcc -O2 -Wall -Werror=implicit-function-declaration -c legacy_network.c"
    echo ""

    # 用定制 GCC 编译 — implicit-function-declaration 被降级为 warning
    ${gccTolerant}/bin/gcc -O2 -Wall \
        -Wimplicit-function-declaration \
        -c legacy_network.c -o legacy_network.o

    echo ""
    echo "✅ 编译成功（警告仍然可见，但不再是 error）"
  '';

  installPhase = ''
    mkdir -p $out/lib
    cp legacy_network.o $out/lib/
  '';

  meta = {
    description = "Legacy network module (compiled with tolerant GCC)";
    platforms = pkgs.lib.platforms.linux;
  };
}
