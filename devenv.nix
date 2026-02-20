{ pkgs, ... }:
{
  languages.rust.enable = true;

  packages = with pkgs; [
    sqlite
    pkg-config
    openssl
    docker-compose
    neo4j
  ];

  enterShell = ''
    export RUST_LOG=${RUST_LOG:-info}
  '';

  scripts.test-all.exec = ''
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all
  '';
}
