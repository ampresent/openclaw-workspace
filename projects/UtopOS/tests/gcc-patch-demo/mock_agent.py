#!/usr/bin/env python3
"""
mock_agent.py — UtopOS-agent 的模拟服务器

模拟 UtopOS-agent 的 API 端点，返回预设的响应数据，
用于在非 NixOS 环境下演示 UtopOS 的完整工作流。

用法：
  python3 mock_agent.py [port]
  默认端口：7890
"""

import json
import sys
import socket
from http.server import HTTPServer, BaseHTTPRequestHandler

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 7890

# ============================================================
# 模拟数据
# ============================================================

SNAPSHOT_FAIL = {
    "hostname": "prod-web-01",
    "nixos_version": "25.05.20260410.abc1234 (Warbler)",
    "kernel": "6.8.0-100-generic",
    "uptime": "5 days, 3 hours",
    "generation": 42,
    "services": [
        {"name": "sshd.service", "active": "active", "sub": "running",
         "description": "OpenSSH server daemon"},
        {"name": "nginx.service", "active": "active", "sub": "running",
         "description": "A high performance web server"},
        {"name": "legacy-network-build.service", "active": "failed", "sub": "failed",
         "description": "Legacy network module build"},
        {"name": "postgresql.service", "active": "active", "sub": "running",
         "description": "PostgreSQL database server"},
    ],
    "disk": [{"mount": "/", "used_pct": 45}],
    "memory": {"total": "16GB", "used": "6.2GB", "available": "9.8GB"},
    "recent_failures": [
        {
            "unit": "legacy-network-build.service",
            "since": "10 minutes ago",
            "log_excerpt": (
                "legacy_network.c:42:5: error: implicit declaration of function 'memcpy'\n"
                "  [-Wimplicit-function-declaration]\n"
                "   42 |     memcpy(dst, src, len);\n"
                "      |     ^~~~~~\n"
                "make: *** [Makefile:12: legacy_network.o] Error 1"
            )
        }
    ]
}

SERVICE_LOGS = {
    "unit": "legacy-network-build.service",
    "logs": [
        "Apr 12 10:00:01 prod-web-01 systemd[1]: Starting Legacy network module build...",
        "Apr 12 10:00:01 prod-web-01 build.sh[12345]: === 编译 legacy_network.c ===",
        "Apr 12 10:00:01 prod-web-01 build.sh[12345]: gcc -O2 -Wall -Werror=implicit-function-declaration -c legacy_network.c",
        "Apr 12 10:00:01 prod-web-01 build.sh[12345]: legacy_network.c:42:5: error: implicit declaration of function 'memcpy'",
        "Apr 12 10:00:01 prod-web-01 build.sh[12345]:   [-Wimplicit-function-declaration]",
        "Apr 12 10:00:01 prod-web-01 build.sh[12345]:    42 |     memcpy(dst, src, len);",
        "Apr 12 10:00:01 prod-web-01 build.sh[12345]:       |     ^~~~~~",
        "Apr 12 10:00:01 prod-web-01 build.sh[12345]: make: *** [Makefile:12: legacy_network.o] Error 1",
        "Apr 12 10:00:01 prod-web-01 systemd[1]: legacy-network-build.service: Main process exited, code=exited, status=2/INVALIDARGUMENT",
        "Apr 12 10:00:01 prod-web-01 systemd[1]: legacy-network-build.service: Failed with result 'exit-code'.",
        "Apr 12 10:00:01 prod-web-01 systemd[1]: Failed to start Legacy network module build.",
    ]
}

CONFIG_READ = {
    "path": "/etc/nixos/configuration.nix",
    "content": (
        "{ config, pkgs, ... }:\n"
        "{\n"
        '  imports = [ ./hardware-configuration.nix ];\n'
        "\n"
        '  boot.loader.systemd-boot.enable = true;\n'
        '  networking.hostName = "prod-web-01";\n'
        '  networking.firewall.allowedTCPPorts = [ 22 80 443 ];\n'
        "\n"
        "  services.openssh.enable = true;\n"
        "  services.nginx.enable = true;\n"
        "  services.postgresql.enable = true;\n"
        "\n"
        "  # legacy-network 模块\n"
        "  systemd.services.legacy-network-build = {\n"
        '    description = "Legacy network module build";\n'
        "    serviceConfig.Type = \"oneshot\";\n"
        "    serviceConfig.ExecStart = ''\n"
        "      ${pkgs.bash}/bin/bash /opt/legacy-network/build.sh\n"
        "    '';\n"
        "    wantedBy = [ \"multi-user.target\" ];\n"
        "  };\n"
        "\n"
        "  environment.systemPackages = with pkgs; [\n"
        "    vim git curl wget\n"
        "  ];\n"
        "\n"
        '  system.stateVersion = "25.05";\n'
        "}\n"
    )
}

CONFIG_VALIDATE_FAIL = {
    "valid": False,
    "dry_run_output": (
        "building the system configuration...\n"
        "error: builder for '/nix/store/xyz-legacy-network-0.1.0.drv' failed with exit code 2;\n"
        "       last 10 log lines:\n"
        "       > legacy_network.c:42:5: error: implicit declaration of function 'memcpy'\n"
        "       >   [-Wimplicit-function-declaration]\n"
        "       >    42 |     memcpy(dst, src, len);\n"
        "       >       |     ^~~~~~\n"
        "       > make: *** [Makefile:12: legacy_network.o] Error 1\n"
        "error: 1 dependencies of derivation '/nix/store/abc-system-path.drv' failed to build\n"
    ),
    "summary": {
        "packages_added": [],
        "packages_removed": [],
        "services_restart": [],
        "services_stop": [],
        "risk_level": "unknown",
        "risk_reasons": ["dry-build 失败，无法评估风险"]
    }
}

CONFIG_VALIDATE_PASS = {
    "valid": True,
    "dry_run_output": (
        "building the system configuration...\n"
        "these derivations will be built:\n"
        "  /nix/store/new-legacy-network-0.1.0-fixed.drv\n"
        "  /nix/store/sys-system-path.drv\n"
        "building '/nix/store/new-legacy-network-0.1.0-fixed.drv'...\n"
        "legacy_network.c:42:5: warning: implicit declaration of function 'memcpy'\n"
        "  [-Wimplicit-function-declaration]\n"
        "   42 |     memcpy(dst, src, len);\n"
        "      |     ^~~~~~\n"
        "note: 'memcpy' is declared in <string.h>\n"
        "building '/nix/store/sys-system-path.drv'...\n"
        "activating configuration...\n"
        "restarting legacy-network-build.service\n"
    ),
    "summary": {
        "packages_added": ["legacy-network-0.1.0-fixed"],
        "packages_removed": [],
        "services_restart": ["legacy-network-build.service"],
        "services_stop": [],
        "risk_level": "safe",
        "risk_reasons": [
            "仅添加配置项，无破坏性变更",
            "变更范围仅限 legacy-network 包"
        ]
    }
}

CONFIG_APPLY = {
    "success": True,
    "generation": 43,
    "summary": "配置已生效：legacy-network 使用容错 GCC 编译成功，implicit-function-declaration 降级为 warning",
    "rollback_command": "nixos-rebuild switch --rollback"
}

GENERATIONS = {
    "current": 43,
    "generations": [
        {"number": 43, "date": "2026-04-12 10:15", "description": "GCC overlay: 容忍 legacy-network implicit-function-declaration"},
        {"number": 42, "date": "2026-04-10 22:00", "description": "初始配置"},
    ]
}

HEALTH = {
    "status": "ok",
    "version": "0.3.1",
    "uptime_seconds": 86400,
    "nixos_detected": True
}


# ============================================================
# HTTP Handler
# ============================================================

class MockHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # 静默日志，不污染 demo 输出
        pass

    def _respond(self, data, status=200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False, indent=2).encode())

    def do_GET(self):
        routes = {
            "/health": HEALTH,
            "/api/snapshot": SNAPSHOT_FAIL,
            "/api/logs?unit=legacy-network-build.service&lines=20": SERVICE_LOGS,
            "/api/config": CONFIG_READ,
            "/api/generations": GENERATIONS,
        }
        if self.path in routes:
            self._respond(routes[self.path])
        elif self.path.startswith("/api/logs"):
            self._respond(SERVICE_LOGS)
        else:
            self._respond({"error": f"not found: {self.path}"}, 404)

    def do_POST(self):
        content_len = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_len).decode() if content_len else "{}"

        if self.path == "/api/config/validate":
            req = json.loads(body)
            # 第一次请求返回失败（初始配置），第二次返回成功（修复后）
            config = req.get("config", "")
            if "Wno-error=implicit-function-declaration" in config or "legacy-network-fixed" in config:
                self._respond(CONFIG_VALIDATE_PASS)
            else:
                self._respond(CONFIG_VALIDATE_FAIL)
        elif self.path == "/api/config/apply":
            self._respond(CONFIG_APPLY)
        elif self.path == "/api/rollback":
            self._respond({"success": True, "reverted_to": 42,
                           "summary": "已回滚到 generation 42"})
        else:
            self._respond({"error": f"not found: {self.path}"}, 404)


if __name__ == "__main__":
    # Allow address reuse to avoid port conflicts
    class ReuseHTTPServer(HTTPServer):
        allow_reuse_address = True
    server = ReuseHTTPServer(("127.0.0.1", PORT), MockHandler)
    print(f"mock-agent listening on 127.0.0.1:{PORT}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
