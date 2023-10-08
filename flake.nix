{
  description = "actix-session-surrealdb development flake";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system: let
        pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          rustup
        ];

        shellHook = ''
          rustup default nightly
          rustup component add rust-analyzer
          cargo help watch 2>/dev/null 1>/dev/null || cargo install cargo-watch
        '';
      };
    });
}
