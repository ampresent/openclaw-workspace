#[cfg(test)]
mod tests {
    use super::*;

    // ─── analyze_config: hostname detection ─────────────────────────

    #[test]
    fn test_detect_hostname_from_config() {
        let config = r#"
{ config, pkgs, ... }:
{
  networking.hostName = "myserver";
  services.nginx.enable = true;
}
"#;
        let analysis = analyze_config(config, None);
        assert_eq!(analysis.hostname, "myserver");
    }

    #[test]
    fn test_detect_hostname_override() {
        let config = r#"{ networking.hostName = "original"; }"#;
        let analysis = analyze_config(config, Some("override"));
        assert_eq!(analysis.hostname, "override");
    }

    #[test]
    fn test_detect_hostname_default() {
        let config = r#"{ services.nginx.enable = true; }"#;
        let analysis = analyze_config(config, None);
        assert_eq!(analysis.hostname, "nixos");
    }

    // ─── analyze_config: service detection ──────────────────────────

    #[test]
    fn test_detect_services() {
        let config = r#"
{
  services.nginx.enable = true;
  services.postgresql.enable = true;
  services.redis.enable = true;
  services.openssh.enable = true;
}
"#;
        let analysis = analyze_config(config, None);
        assert!(analysis.services.contains(&"nginx".to_string()));
        assert!(analysis.services.contains(&"postgresql".to_string()));
        assert!(analysis.services.contains(&"redis".to_string()));
        assert!(analysis.services.contains(&"openssh".to_string()));
    }

    #[test]
    fn test_no_services_detected() {
        let config = r#"{ environment.systemPackages = [ pkgs.vim ]; }"#;
        let analysis = analyze_config(config, None);
        assert!(analysis.services.is_empty());
    }

    // ─── analyze_config: legacy references ──────────────────────────

    #[test]
    fn test_detect_legacy_nixpkgs() {
        let config = r#"
{
  environment.systemPackages = [ <nixpkgs>.pkgs.vim ];
}
"#;
        let analysis = analyze_config(config, None);
        assert!(analysis.has_legacy_refs);
        assert!(analysis.warnings.iter().any(|w| w.contains("<nixpkgs>")));
    }

    #[test]
    fn test_no_legacy_refs_modern_config() {
        let config = r#"
{ pkgs, ... }:
{
  environment.systemPackages = [ pkgs.vim ];
}
"#;
        let analysis = analyze_config(config, None);
        assert!(!analysis.has_legacy_refs);
    }

    // ─── analyze_config: hardware module ────────────────────────────

    #[test]
    fn test_detect_hardware_config() {
        let config = r#"
{
  imports = [
    ./hardware-configuration.nix
  ];
}
"#;
        let analysis = analyze_config(config, None);
        assert!(analysis.hardware_modules.contains(&"hardware-configuration.nix".to_string()));
    }

    // ─── analyze_config: home-manager ───────────────────────────────

    #[test]
    fn test_detect_home_manager() {
        let config = r#"
{
  imports = [
    <home-manager/nixos>
  ];
}
"#;
        let analysis = analyze_config(config, None);
        assert!(analysis.imports.contains(&"home-manager".to_string()));
    }

    // ─── generate_flake_nix ─────────────────────────────────────────

    #[test]
    fn test_flake_output_has_description() {
        let analysis = ConfigAnalysis {
            imports: vec![],
            services: vec!["nginx".into()],
            hardware_modules: vec![],
            hostname: "testhost".into(),
            has_legacy_refs: false,
            has_overlays: false,
            warnings: vec![],
        };
        let inputs = std::collections::HashMap::new();
        let flake = generate_flake_nix("nixos-24.05", "testhost", &analysis, &inputs);

        assert!(flake.contains("description = "NixOS configuration for testhost""));
    }

    #[test]
    fn test_flake_output_has_nixpkgs_input() {
        let analysis = ConfigAnalysis {
            imports: vec![], services: vec![], hardware_modules: vec![],
            hostname: "host".into(), has_legacy_refs: false,
            has_overlays: false, warnings: vec![],
        };
        let inputs = std::collections::HashMap::new();
        let flake = generate_flake_nix("nixos-24.05", "host", &analysis, &inputs);

        assert!(flake.contains("nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05""));
    }

    #[test]
    fn test_flake_output_has_nixos_system() {
        let analysis = ConfigAnalysis {
            imports: vec![], services: vec![], hardware_modules: vec![],
            hostname: "myhost".into(), has_legacy_refs: false,
            has_overlays: false, warnings: vec![],
        };
        let inputs = std::collections::HashMap::new();
        let flake = generate_flake_nix("nixos-24.05", "myhost", &analysis, &inputs);

        assert!(flake.contains("nixosConfigurations.myhost"));
        assert!(flake.contains("nixpkgs.lib.nixosSystem"));
        assert!(flake.contains("./configuration.nix"));
    }

    #[test]
    fn test_flake_with_extra_inputs() {
        let analysis = ConfigAnalysis {
            imports: vec![], services: vec![], hardware_modules: vec![],
            hostname: "h".into(), has_legacy_refs: false,
            has_overlays: false, warnings: vec![],
        };
        let mut inputs = std::collections::HashMap::new();
        inputs.insert("agenix".into(), "github:ryantm/agenix".into());
        let flake = generate_flake_nix("nixos-24.05", "h", &analysis, &inputs);

        assert!(flake.contains("agenix.url = "github:ryantm/agenix""));
    }

    #[test]
    fn test_flake_with_home_manager() {
        let analysis = ConfigAnalysis {
            imports: vec!["home-manager".into()],
            services: vec![], hardware_modules: vec![],
            hostname: "hm".into(), has_legacy_refs: false,
            has_overlays: false, warnings: vec![],
        };
        let inputs = std::collections::HashMap::new();
        let flake = generate_flake_nix("nixos-24.05", "hm", &analysis, &inputs);

        assert!(flake.contains("home-manager.url"));
        assert!(flake.contains("home-manager.nixosModules.home-manager"));
    }

    #[test]
    fn test_flake_channel_no_double_prefix() {
        let analysis = ConfigAnalysis {
            imports: vec![], services: vec![], hardware_modules: vec![],
            hostname: "x".into(), has_legacy_refs: false,
            has_overlays: false, warnings: vec![],
        };
        let inputs = std::collections::HashMap::new();
        // Already has nixos- prefix
        let flake = generate_flake_nix("nixos-24.05", "x", &analysis, &inputs);
        assert!(flake.contains("nixpkgs/nixos-24.05"));
        assert!(!flake.contains("nixpkgs/nixos-nixos-"));

        // Without prefix
        let flake2 = generate_flake_nix("24.05", "x", &analysis, &inputs);
        assert!(flake2.contains("nixpkgs/24.05"));
    }

    // ─── ConfigAnalysis complete scenario ───────────────────────────

    #[test]
    fn test_full_config_analysis() {
        let config = r#"
{ config, pkgs, ... }:
{
  imports = [
    ./hardware-configuration.nix
  ];
  networking.hostName = "prod-server";
  services.nginx.enable = true;
  services.postgresql.enable = true;
  services.openssh.enable = true;
}
"#;
        let analysis = analyze_config(config, None);

        assert_eq!(analysis.hostname, "prod-server");
        assert!(analysis.services.contains(&"nginx".to_string()));
        assert!(analysis.services.contains(&"postgresql".to_string()));
        assert!(analysis.services.contains(&"openssh".to_string()));
        assert!(analysis.hardware_modules.contains(&"hardware-configuration.nix".to_string()));
        assert!(!analysis.has_legacy_refs);
        assert!(analysis.warnings.is_empty());
    }
}
