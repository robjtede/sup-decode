{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    x52 = {
      url = "github:x52dev/nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.x52.flakeModules.default ];

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      perSystem = { pkgs, config, lib, ... }: {
        formatter = pkgs.nixpkgs-fmt;

        devShells.default = pkgs.mkShell {
          packages = [
            config.formatter
            pkgs.cargo-nextest
            pkgs.fd
            pkgs.jq
            pkgs.just
            pkgs.prettier
            pkgs.taplo
          ] ++ lib.optional pkgs.stdenv.isDarwin [
            pkgs.pkgsBuildHost.libiconv
          ];

          shellHook = config.x52.justRust.shellHook;
        };
      };
    };
}
