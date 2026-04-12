# legacy-network.nix — 尝试编译 legacy_network.c
#
# 这个表达式在 GCC 14+ 上会编译失败，因为：
#   1. 代码遗漏了 #include <string.h>
#   2. GCC 14 将 -Wimplicit-function-declaration 默认升级为 error
#
# 编译失败输出：
#   legacy_network.c:42:5: error: implicit declaration of function 'memcpy'
#
# 这就是用户遇到的编译问题 — 需要通过 UtopOS 诊断并修复

{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation {
  pname = "legacy-network";
  version = "0.1.0";

  src = ./.;

  buildInputs = [ pkgs.gcc ];

  buildPhase = ''
    echo "=== 编译 legacy_network.c ==="
    gcc -O2 -Wall -Werror=implicit-function-declaration \
        -c legacy_network.c -o legacy_network.o
  '';

  installPhase = ''
    mkdir -p $out/lib
    cp legacy_network.o $out/lib/
  '';

  meta = {
    description = "Legacy network module (expected to fail on GCC 14+)";
    platforms = pkgs.lib.platforms.linux;
  };
}
