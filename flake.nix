{
  description = "Mattak supports semantic web applications in Axum";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = (
          import "${nixpkgs}" {
            inherit system overlays;
          }
        );

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # (rust-bin.selectLatestNightlyWith ( toolchain: toolchain.default))
            # .override { extensions = [ "rust-analyzer" ]; }
            (rust-bin.stable.latest.default.override {
              extensions = [
                "rust-analyzer"
                "rust-src"
              ];
            })
            #cargo-expand
            yj
            jq
          ];
        };
      }
    );
}
