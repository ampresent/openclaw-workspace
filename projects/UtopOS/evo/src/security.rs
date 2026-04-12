/// Security Scanner — Scan configuration.nix for security issues
///
/// Checks: open ports, weak passwords, outdated packages,
/// missing firewall rules, dangerous permissions, CVE lookups.

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::AppError;
use crate::cmd;

// ─── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SecurityReport {
    pub scan_time: String,
    pub hostname: String,
    pub score: u8, // 0-100
    pub findings: Vec<Finding>,
    pub summary: ScanSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub line_hint: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanSummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub open_ports: Vec<u16>,
    pub services_checked: usize,
    pub packages_checked: usize,
}

// ─── Scanners ─────────────────────────────────────────────────────────────

pub async fn scan_config(config_path: &str) -> Result<SecurityReport, AppError> {
    let config_content = cmd::run_cmd("cat", &[config_path]).await
        .unwrap_or_else(|_| String::new());

    let hostname = cmd::run_cmd("hostname", &[]).await
        .unwrap_or_else(|_| "unknown".into());

    let mut findings = Vec::new();

    // 1. Firewall checks
    check_firewall(&config_content, &mut findings);

    // 2. Open ports
    let open_ports = check_open_ports(&mut findings).await;

    // 3. SSH security
    check_ssh_security(&config_content, &mut findings);

    // 4. Password and auth
    check_auth_security(&config_content, &mut findings);

    // 5. Service security
    check_service_security(&config_content, &mut findings);

    // 6. Package security
    let pkg_count = check_package_security(&mut findings).await;

    // 7. File permissions
    check_file_permissions(&mut findings).await;

    // 8. Kernel security
    check_kernel_security(&config_content, &mut findings);

    // Compute summary
    let summary = ScanSummary {
        critical: findings.iter().filter(|f| matches!(f.severity, Severity::Critical)).count(),
        high: findings.iter().filter(|f| matches!(f.severity, Severity::High)).count(),
        medium: findings.iter().filter(|f| matches!(f.severity, Severity::Medium)).count(),
        low: findings.iter().filter(|f| matches!(f.severity, Severity::Low)).count(),
        info: findings.iter().filter(|f| matches!(f.severity, Severity::Info)).count(),
        open_ports: open_ports.clone(),
        services_checked: count_services(&config_content),
        packages_checked: pkg_count,
    };

    // Score: 100 - penalties
    let score = (100i32
        - (summary.critical as i32) * 20
        - (summary.high as i32) * 10
        - (summary.medium as i32) * 5
        - (summary.low as i32) * 2)
        .clamp(0, 100) as u8;

    Ok(SecurityReport {
        scan_time: chrono::Utc::now().to_rfc3339(),
        hostname,
        score,
        findings,
        summary,
    })
}

fn check_firewall(config: &str, findings: &mut Vec<Finding>) {
    if !config.contains("networking.firewall.enable") {
        findings.push(Finding {
            severity: Severity::Critical,
            category: "firewall".into(),
            title: "防火墙未配置".into(),
            description: "configuration.nix 中没有找到 networking.firewall.enable 设置。".into(),
            recommendation: "添加 networking.firewall.enable = true; 并配置允许的端口。".into(),
            line_hint: find_line(config, "networking.firewall"),
        });
    } else if config.contains("networking.firewall.enable = false") {
        findings.push(Finding {
            severity: Severity::Critical,
            category: "firewall".into(),
            title: "防火墙已禁用".into(),
            description: "防火墙被显式禁用，系统对所有入站连接开放。".into(),
            recommendation: "设置 networking.firewall.enable = true;".into(),
            line_hint: find_line(config, "networking.firewall.enable = false"),
        });
    }

    if !config.contains("firewall.allowedTCPPorts") && !config.contains("firewall.allowedUDPPorts") {
        findings.push(Finding {
            severity: Severity::Medium,
            category: "firewall".into(),
            title: "未配置防火墙端口规则".into(),
            description: "没有检测到端口白名单配置。".into(),
            recommendation: "使用 networking.firewall.allowedTCPPorts 显式声明需要开放的端口。".into(),
            line_hint: None,
        });
    }
}

async fn check_open_ports(findings: &mut Vec<Finding>) -> Vec<u16> {
    let mut ports = Vec::new();

    // Check with ss
    if let Ok(output) = cmd::run_cmd("bash", &["-c", "ss -tlnp 2>/dev/null | grep LISTEN | awk '{print $4}' | grep -oE '[0-9]+$' | sort -un"]).await {
        for line in output.lines() {
            if let Ok(port) = line.trim().parse::<u16>() {
                ports.push(port);
            }
        }
    }

    // Flag dangerous ports
    let dangerous: HashMap<u16, &str> = HashMap::from([
        (21, "FTP — 使用 SFTP 替代"),
        (23, "Telnet — 使用 SSH 替代"),
        (111, "RPC — 考虑禁用"),
        (445, "SMB — 确认是否需要公开"),
        (3306, "MySQL — 限制为本地访问"),
        (5432, "PostgreSQL — 限制为本地访问"),
        (6379, "Redis — 限制为本地访问，设置密码"),
        (27017, "MongoDB — 限制为本地访问，设置认证"),
    ]);

    for &port in &ports {
        if let Some(reason) = dangerous.get(&port) {
            findings.push(Finding {
                severity: Severity::High,
                category: "ports".into(),
                title: format!("危险端口 {port} 开放"),
                description: format!("端口 {port} 正在监听：{reason}"),
                recommendation: format!("确认端口 {port} 是否必须对外开放，如不需要请关闭。"),
                line_hint: None,
            });
        }
    }

    // Check for services listening on 0.0.0.0
    if let Ok(output) = cmd::run_cmd("bash", &["-c", "ss -tlnp 2>/dev/null | grep '0.0.0.0' | awk '{print $4}'"]).await {
        for line in output.lines() {
            if let Some(port_str) = line.split(':').last() {
                if let Ok(_port) = port_str.trim().parse::<u16>() {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: "binding".into(),
                        title: format!("服务监听 0.0.0.0:{port_str}"),
                        description: "服务绑定到所有接口，包括公网。".into(),
                        recommendation: "如仅需本地访问，改为绑定 127.0.0.1。".into(),
                        line_hint: None,
                    });
                }
            }
        }
    }

    ports
}

fn check_ssh_security(config: &str, findings: &mut Vec<Finding>) {
    if !config.contains("services.openssh") {
        findings.push(Finding {
            severity: Severity::Info,
            category: "ssh".into(),
            title: "SSH 服务未配置".into(),
            description: "未在 configuration.nix 中发现 OpenSSH 配置。".into(),
            recommendation: "如需远程访问，配置 services.openssh.enable = true; 并做好安全加固。".into(),
            line_hint: None,
        });
        return;
    }

    if !config.contains("PermitRootLogin") && !config.contains("permitRootLogin") {
        findings.push(Finding {
            severity: Severity::High,
            category: "ssh".into(),
            title: "SSH root 登录未限制".into(),
            description: "未设置 PermitRootLogin，root 可能可以通过 SSH 直接登录。".into(),
            recommendation: "添加 services.openssh.settings.PermitRootLogin = \"no\";".into(),
            line_hint: find_line(config, "openssh"),
        });
    }

    if !config.contains("PasswordAuthentication") && !config.contains("passwordAuthentication") {
        findings.push(Finding {
            severity: Severity::Medium,
            category: "ssh".into(),
            title: "SSH 密码认证未禁用".into(),
            description: "SSH 密码认证可能开启，容易受到暴力破解攻击。".into(),
            recommendation: "设置 services.openssh.settings.PasswordAuthentication = false; 并使用密钥认证。".into(),
            line_hint: find_line(config, "openssh"),
        });
    }

    if config.contains("PermitRootLogin = true") || config.contains("PermitRootLogin = \"yes\"") {
        findings.push(Finding {
            severity: Severity::Critical,
            category: "ssh".into(),
            title: "SSH 允许 root 登录".into(),
            description: "SSH 被配置为允许 root 直接登录，这是严重的安全隐患。".into(),
            recommendation: "设置 services.openssh.settings.PermitRootLogin = \"no\";".into(),
            line_hint: find_line(config, "PermitRootLogin"),
        });
    }
}

fn check_auth_security(config: &str, findings: &mut Vec<Finding>) {
    // Check for empty passwords
    if config.contains("users.users") && config.contains("password = \"\"") {
        findings.push(Finding {
            severity: Severity::Critical,
            category: "auth".into(),
            title: "用户设置了空密码".into(),
            description: "检测到用户使用空密码，任何人都可以登录。".into(),
            recommendation: "使用 hashedPassword 替代 password，并设置强密码。".into(),
            line_hint: find_line(config, "password = \"\""),
        });
    }

    // Check for weak/default passwords
    let weak_passwords = ["password", "123456", "admin", "root", "test", "guest"];
    for weak in &weak_passwords {
        if config.contains(&format!("password = \"{weak}\"")) {
            findings.push(Finding {
                severity: Severity::Critical,
                category: "auth".into(),
                title: format!("弱密码检测: {weak}"),
                description: format!("用户使用了弱密码 '{weak}'，极易被破解。"),
                recommendation: "立即更改密码，使用至少 12 位包含大小写、数字和特殊字符的强密码。".into(),
                line_hint: find_line(config, &format!("password = \"{weak}\"")),
            });
        }
    }

    // Check passwordless sudo
    if config.contains("security.sudo.wheelNeedsPassword = false") {
        findings.push(Finding {
            severity: Severity::Medium,
            category: "auth".into(),
            title: "wheel 组免密 sudo".into(),
            description: "wheel 组用户执行 sudo 不需要密码。".into(),
            recommendation: "在生产环境中设置 security.sudo.wheelNeedsPassword = true;".into(),
            line_hint: find_line(config, "wheelNeedsPassword = false"),
        });
    }
}

fn check_service_security(config: &str, findings: &mut Vec<Finding>) {
    // Docker security
    if config.contains("virtualisation.docker.enable = true") {
        findings.push(Finding {
            severity: Severity::Medium,
            category: "services".into(),
            title: "Docker 已启用".into(),
            description: "Docker daemon 以 root 运行，容器逃逸可能导致主机被完全控制。".into(),
            recommendation: "考虑使用 rootless Docker（virtualisation.docker.rootless）或 Podman。".into(),
            line_hint: find_line(config, "docker"),
        });
    }

    // NFS exports
    if config.contains("services.nfs.server") && config.contains("export") {
        findings.push(Finding {
            severity: Severity::High,
            category: "services".into(),
            title: "NFS 服务已配置".into(),
            description: "NFS 导出可能允许未授权访问文件系统。".into(),
            recommendation: "确认导出列表限制了访问 IP，并使用 Kerberos 认证。".into(),
            line_hint: find_line(config, "nfs"),
        });
    }

    // Mail services
    if config.contains("services.postfix") || config.contains("services.dovecot") {
        findings.push(Finding {
            severity: Severity::Info,
            category: "services".into(),
            title: "邮件服务已配置".into(),
            description: "邮件服务需要额外的安全配置（TLS、SPF、DKIM 等）。".into(),
            recommendation: "确保邮件服务配置了 TLS 加密和适当的反垃圾邮件措施。".into(),
            line_hint: find_line(config, "postfix"),
        });
    }

    // Web server without TLS
    if (config.contains("services.nginx") || config.contains("services.apache")) && !config.contains("ssl") && !config.contains("tls") && !config.contains("enableACME") {
        findings.push(Finding {
            severity: Severity::Medium,
            category: "services".into(),
            title: "Web 服务器可能未配置 TLS".into(),
            description: "Web 服务器配置中未检测到 SSL/TLS 设置。".into(),
            recommendation: "配置 SSL 证书（使用 ACME 或手动），确保 HTTPS 通信加密。".into(),
            line_hint: find_line(config, "nginx"),
        });
    }
}

async fn check_package_security(findings: &mut Vec<Finding>) -> usize {
    let mut count = 0;

    // Get installed packages
    if let Ok(output) = cmd::run_cmd("bash", &["-c", "nix-store -qR /run/current-system 2>/dev/null | wc -l"]).await {
        count = output.trim().parse().unwrap_or(0);
    }

    // Check for known vulnerable packages (simplified — real impl would use vulnix or nix-audit)
    let vuln_patterns = [
        ("openssl-1.0", "OpenSSL 1.0 已停止维护，存在已知漏洞"),
        ("openssh-7.", "OpenSSH 7.x 存在已知安全问题"),
        ("bash-4.3", "Bash 4.3 存在 Shellshock 漏洞"),
    ];

    if let Ok(output) = cmd::run_cmd("bash", &["-c", "nix-store -qR /run/current-system 2>/dev/null | head -1000"]).await {
        for (pattern, desc) in &vuln_patterns {
            if output.contains(pattern) {
                findings.push(Finding {
                    severity: Severity::High,
                    category: "packages".into(),
                    title: format!("潜在漏洞包: {pattern}"),
                    description: desc.to_string(),
                    recommendation: format!("升级 {pattern} 到最新版本。"),
                    line_hint: None,
                });
            }
        }
    }

    count
}

async fn check_file_permissions(findings: &mut Vec<Finding>) {
    // Check /etc/nixos permissions
    if let Ok(output) = cmd::run_cmd("bash", &["-c", "stat -c '%a %n' /etc/nixos/configuration.nix 2>/dev/null"]).await {
        let parts: Vec<&str> = output.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == "666" || parts[0] == "644" || parts[0] == "664" {
            // These are actually common, only flag truly bad ones
            if parts[0] == "666" {
                findings.push(Finding {
                    severity: Severity::Medium,
                    category: "permissions".into(),
                    title: "配置文件权限过于宽松".into(),
                    description: format!("/etc/nixos/configuration.nix 权限为 {}，所有用户可写。", parts[0]),
                    recommendation: "设置为 644 或更严格的权限：chmod 644 /etc/nixos/configuration.nix".into(),
                    line_hint: None,
                });
            }
        }
    }

    // Check for world-writable files in /etc
    if let Ok(output) = cmd::run_cmd("bash", &["-c", "find /etc -perm -o+w -type f 2>/dev/null | head -5"]).await {
        if !output.trim().is_empty() {
            for line in output.lines() {
                findings.push(Finding {
                    severity: Severity::Medium,
                    category: "permissions".into(),
                    title: "全局可写文件".into(),
                    description: format!("{line} 对所有用户可写"),
                    recommendation: "移除 world-writable 权限：chmod o-w <file>".into(),
                    line_hint: None,
                });
            }
        }
    }
}

fn check_kernel_security(config: &str, findings: &mut Vec<Finding>) {
    if !config.contains("security.apparmor") && !config.contains("security.selinux") {
        findings.push(Finding {
            severity: Severity::Medium,
            category: "kernel".into(),
            title: "未配置 MAC 框架".into(),
            description: "未检测到 AppArmor 或 SELinux 等强制访问控制框架。".into(),
            recommendation: "考虑启用 security.apparmor.enable = true; 增加系统安全层。".into(),
            line_hint: None,
        });
    }

    if config.contains("boot.kernel.sysctl") {
        if config.contains("net.ipv4.conf.all.forwarding = 1") {
            findings.push(Finding {
                severity: Severity::Low,
                category: "kernel".into(),
                title: "IP 转发已启用".into(),
                description: "系统配置为路由器模式，可能被用于网络跳板。".into(),
                recommendation: "如非路由器/VPN 网关，禁用 IP 转发。".into(),
                line_hint: find_line(config, "forwarding"),
            });
        }
    }
}

fn count_services(config: &str) -> usize {
    config.lines().filter(|l| l.contains("services.") && l.contains(".enable = true")).count()
}

fn find_line(content: &str, pattern: &str) -> Option<usize> {
    for (i, line) in content.lines().enumerate() {
        if line.contains(pattern) {
            return Some(i + 1);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── find_line ──────────────────────────────────────────────────

    #[test]
    fn test_find_line_found() {
        let content = "line1\nline2\nmatch here\nline4";
        assert_eq!(find_line(content, "match"), Some(3));
    }

    #[test]
    fn test_find_line_not_found() {
        let content = "line1\nline2\nline3";
        assert_eq!(find_line(content, "missing"), None);
    }

    #[test]
    fn test_find_line_first_line() {
        let content = "first\nsecond";
        assert_eq!(find_line(content, "first"), Some(1));
    }

    #[test]
    fn test_find_line_empty_content() {
        assert_eq!(find_line("", "anything"), None);
    }

    // ─── count_services ─────────────────────────────────────────────

    #[test]
    fn test_count_services_zero() {
        let config = "{ environment.systemPackages = [ pkgs.vim ]; }";
        assert_eq!(count_services(config), 0);
    }

    #[test]
    fn test_count_services_three() {
        let config = r#"{
            services.nginx.enable = true;
            services.postgresql.enable = true;
            services.redis.enable = true;
        }"#;
        assert_eq!(count_services(config), 3);
    }

    #[test]
    fn test_count_services_disabled_not_counted() {
        let config = "{ services.nginx.enable = false; }";
        assert_eq!(count_services(config), 0);
    }

    // ─── check_firewall ─────────────────────────────────────────────

    #[test]
    fn test_firewall_missing_triggers_critical() {
        let config = "{ services.nginx.enable = true; }";
        let mut findings = vec![];
        check_firewall(config, &mut findings);
        assert!(findings.iter().any(|f| f.category == "firewall" && matches!(f.severity, Severity::Critical)));
    }

    #[test]
    fn test_firewall_disabled_triggers_critical() {
        let config = "{ networking.firewall.enable = false; }";
        let mut findings = vec![];
        check_firewall(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("禁用")));
    }

    #[test]
    fn test_firewall_enabled_no_critical() {
        let config = r#"{
            networking.firewall.enable = true;
            networking.firewall.allowedTCPPorts = [ 80 443 ];
        }"#;
        let mut findings = vec![];
        check_firewall(config, &mut findings);
        assert!(!findings.iter().any(|f| matches!(f.severity, Severity::Critical)));
    }

    #[test]
    fn test_firewall_no_port_rules_triggers_medium() {
        let config = "{ networking.firewall.enable = true; }";
        let mut findings = vec![];
        check_firewall(config, &mut findings);
        assert!(findings.iter().any(|f| f.category == "firewall" && matches!(f.severity, Severity::Medium)));
    }

    // ─── check_ssh_security ─────────────────────────────────────────

    #[test]
    fn test_ssh_not_configured() {
        let config = "{ services.nginx.enable = true; }";
        let mut findings = vec![];
        check_ssh_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("SSH 服务未配置")));
    }

    #[test]
    fn test_ssh_root_login_permit_is_critical() {
        let config = r#"{
            services.openssh.enable = true;
            services.openssh.settings.PermitRootLogin = "yes";
        }"#;
        let mut findings = vec![];
        check_ssh_security(config, &mut findings);
        assert!(findings.iter().any(|f| matches!(f.severity, Severity::Critical) && f.title.contains("root 登录")));
    }

    #[test]
    fn test_ssh_no_permit_root_login_setting() {
        let config = "{ services.openssh.enable = true; }";
        let mut findings = vec![];
        check_ssh_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("root 登录未限制")));
    }

    #[test]
    fn test_ssh_no_password_auth_setting() {
        let config = r#"{
            services.openssh.enable = true;
            services.openssh.settings.PermitRootLogin = "no";
        }"#;
        let mut findings = vec![];
        check_ssh_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("密码认证未禁用")));
    }

    #[test]
    fn test_ssh_hardened_no_findings() {
        let config = r#"{
            services.openssh.enable = true;
            services.openssh.settings.PermitRootLogin = "no";
            services.openssh.settings.PasswordAuthentication = false;
        }"#;
        let mut findings = vec![];
        check_ssh_security(config, &mut findings);
        assert!(findings.is_empty());
    }

    // ─── check_auth_security ────────────────────────────────────────

    #[test]
    fn test_auth_empty_password() {
        let config = r#"{
            users.users.alice = {
                password = "";
                isNormalUser = true;
            };
        }"#;
        let mut findings = vec![];
        check_auth_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("空密码")));
    }

    #[test]
    fn test_auth_weak_password() {
        let config = r#"{
            users.users.alice = {
                password = "admin";
                isNormalUser = true;
            };
        }"#;
        let mut findings = vec![];
        check_auth_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("弱密码")));
    }

    #[test]
    fn test_auth_passwordless_sudo() {
        let config = "{ security.sudo.wheelNeedsPassword = false; }";
        let mut findings = vec![];
        check_auth_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("wheel") && f.title.contains("免密")));
    }

    #[test]
    fn test_auth_strong_password_no_findings() {
        let config = r#"{
            users.users.alice = {
                hashedPassword = "$6$rounds=656000$...";
                isNormalUser = true;
            };
        }"#;
        let mut findings = vec![];
        check_auth_security(config, &mut findings);
        assert!(!findings.iter().any(|f| f.category == "auth"));
    }

    // ─── check_service_security ─────────────────────────────────────

    #[test]
    fn test_service_docker_enabled() {
        let config = "{ virtualisation.docker.enable = true; }";
        let mut findings = vec![];
        check_service_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("Docker")));
    }

    #[test]
    fn test_service_nginx_no_tls() {
        let config = "{ services.nginx.enable = true; }";
        let mut findings = vec![];
        check_service_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("TLS")));
    }

    #[test]
    fn test_service_nginx_with_ssl_no_tls_finding() {
        let config = r#"{
            services.nginx.enable = true;
            services.nginx.sslCertificate = "/etc/ssl/cert.pem";
        }"#;
        let mut findings = vec![];
        check_service_security(config, &mut findings);
        assert!(!findings.iter().any(|f| f.title.contains("TLS")));
    }

    // ─── check_kernel_security ──────────────────────────────────────

    #[test]
    fn test_kernel_no_mac_framework() {
        let config = "{ services.nginx.enable = true; }";
        let mut findings = vec![];
        check_kernel_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("MAC")));
    }

    #[test]
    fn test_kernel_apparmor_configured() {
        let config = "{ security.apparmor.enable = true; }";
        let mut findings = vec![];
        check_kernel_security(config, &mut findings);
        assert!(!findings.iter().any(|f| f.title.contains("MAC")));
    }

    #[test]
    fn test_kernel_ip_forwarding() {
        let config = r#"{
            boot.kernel.sysctl = {
                "net.ipv4.conf.all.forwarding" = 1;
            };
        }"#;
        let mut findings = vec![];
        check_kernel_security(config, &mut findings);
        assert!(findings.iter().any(|f| f.title.contains("IP 转发")));
    }

    // ─── Severity ordering ──────────────────────────────────────────

    #[test]
    fn test_finding_severity_serialization() {
        let finding = Finding {
            severity: Severity::Critical,
            category: "test".into(),
            title: "Test".into(),
            description: "Test".into(),
            recommendation: "Test".into(),
            line_hint: Some(42),
        };
        let json = serde_json::to_string(&finding).unwrap();
        assert!(json.contains("\"critical\""));
    }
}

// ─── API Handlers ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ScanQuery {
    pub config_path: Option<String>,
}

pub async fn handle_scan(Query(q): Query<ScanQuery>) -> Result<impl IntoResponse, AppError> {
    let path = q.config_path.as_deref().unwrap_or("/etc/nixos/configuration.nix");
    let report = scan_config(path).await?;
    Ok(Json(report))
}

pub async fn handle_score() -> Result<impl IntoResponse, AppError> {
    let report = scan_config("/etc/nixos/configuration.nix").await?;
    Ok(Json(serde_json::json!({
        "score": report.score,
        "hostname": report.hostname,
        "critical": report.summary.critical,
        "high": report.summary.high,
        "medium": report.summary.medium,
        "low": report.summary.low,
    })))
}
