let
  pinned = import ./pinned.nix;
in
{ pkgs ? pinned }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    yj
    jq
  ];
  # Enable printing backtraces for rust binaries
  RUST_BACKTRACE = 1;
}
