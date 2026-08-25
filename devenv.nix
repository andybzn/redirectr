{
  pkgs,
  config,
  ...
}:

{
  # Environment
  env.DATABASE_URL = config.secretspec.secrets.DATABASE_URL;
  env.ADMIN_TOKEN = config.secretspec.secrets.ADMIN_TOKEN;

  # Packages
  packages = with pkgs; [
    git
    cargo-audit
    cargo-auditable
    cargo-flamegraph
    cargo-llvm-cov
    bacon
    lldb
    sqlite
    rustup
    sqlx-cli
    nil
    nixd
  ];

  # Languages
  languages.rust = {
    enable = true;
    channel = "stable";
    version = "latest";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "rust-src"
      "llvm-tools-preview"
    ];
  };

  # Processes
  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # Services
  # https://devenv.sh/services/

  # Scripts
  # https://devenv.sh/scripts/
  # scripts.hello.exec = ''
  #   echo hello from $GREET
  # '';

  # Shell
  enterShell = ''
    git --version # Use packages
  '';

  # Tasks
  # https://devenv.sh/tasks/
  tasks = {
    "db:prepare" = {
      exec = ''
        cargo sqlx database create
        cargo sqlx migrate run
        cargo sqlx prepare
      '';
      showOutput = true;
    };
    "app:tests" = {
      # after = ["db:prepare"];
      exec = ''
        cargo fmt --check
        cargo clippy --all-targets -- -D warnings
        cargo test
      '';
    };
  };

  # Tests
  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    devenv tasks run app:tests
  '';

  # Claude support
  claude.code.enable = true;

  # Git hooks
  git-hooks.hooks = {
    clippy.enable = true;
    rustfmt.enable = true;
    # gitleaks = {
    #   name = "gitleaks";
    #   enable = true;
    #   description = "Run gitleaks on the project";
    #   entry = "${pkgs.gitleaks}/bin/gitleaks git";
    # };
  };

  # Outputs
  outputs = {
    rust-app = config.languages.rust.import ./. { };
  };
}
