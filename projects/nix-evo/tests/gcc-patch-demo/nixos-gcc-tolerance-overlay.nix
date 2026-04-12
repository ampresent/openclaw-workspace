# nixos-gcc-tolerance-overlay.nix — NixOS 配置片段
#
# 这是 nix-evo config_apply 会注入到 configuration.nix 的内容。
# 效果：为 legacy-network 包使用定制 GCC，将隐式声明错误降级为警告。
#
# 风险评估：low — 只影响 legacy-network 包的编译，不影响系统 GCC

{ config, pkgs, lib, ... }:

{
  # 方法 A：Per-package overlay（推荐）
  # 只对 legacy-network 使用容错 GCC，不影响其他包
  nixpkgs.overlays = [
    (final: prev: {
      legacy-network = prev.legacy-network.override {
        stdenv = prev.stdenvAdapters.overrideCC prev.stdenv
          (prev.gcc.overrideAttrs (old: {
            NIX_CFLAGS_COMPILE = (old.NIX_CFLAGS_COMPILE or "") +
              " -Wno-error=implicit-function-declaration";
          }));
      };
    })
  ];

  # 方法 B：如果 legacy-network 不是独立包，而是某个 larger package 的一部分，
  # 可以用 packageOverrides：
  #
  # environment.systemPackages = [
  #   (pkgs.callPackage ./legacy-network-fixed.nix {})
  # ];

  # 方法 C：全局 GCC overlay（最后手段 — 影响所有包的编译）
  # 仅在方法 A/B 不可行时使用：
  #
  # nixpkgs.overlays = [
  #   (final: prev: {
  #     gcc = prev.gcc.overrideAttrs (old: {
  #       NIX_CFLAGS_COMPILE = (old.NIX_CFLAGS_COMPILE or "") +
  #         " -Wno-error=implicit-function-declaration";
  #     });
  #   })
  # ];
}
