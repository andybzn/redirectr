{
  description = "Redirectr - a shortlink redirection tool";

  inputs.nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";

  outputs =
    { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;
      cargoToml = fromTOML (builtins.readFile ./Cargo.toml);

      forAllSystems = lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      mkRedirectr =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          SQLX_OFFLINE = "true";

          meta = {
            description = "redirect shortlinks to their target url";
            license = lib.licenses.mit;
            mainProgram = "redirectr";
          };
        };

      mkDockerImage =
        pkgs: package:
        pkgs.dockerTools.buildLayeredImage {
          name = package.pname;
          tag = package.version;
          contents = [ package ];
          fakeRootCommands = ''
            mkdir -p data
            chown 1000:1000 data
          '';
          enableFakechroot = true;
          config = {
            User = "1000:1000";
            Env = [
              "SERVER_IP=0.0.0.0"
              "DATABASE_URL=sqlite:///data/redirectr.sqlite?mode=rwc"
            ];
            Cmd = [ (lib.getExe package) ];
          };
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        rec {
          redirectr = mkRedirectr pkgs;
          default = redirectr;
        }
        // lib.optionalAttrs (system == "x86_64-linux") (
          lib.concatMapAttrs
            (
              target: staticPkgs:
              let
                package = mkRedirectr staticPkgs;
              in
              {
                "redirectr-${target}" = package;
                "dockerImage-${target}" = mkDockerImage staticPkgs package;
              }
            )
            {
              x86_64-linux = pkgs.pkgsStatic;
              aarch64-linux = pkgs.pkgsCross.aarch64-multiplatform.pkgsStatic;
            }
        )
      );
    };
}
