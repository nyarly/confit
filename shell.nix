let
  pinned = import ./pinned.nix;
in
{ pkgs ? pinned }:

pkgs.mkShell {

  buildInputs = with pkgs; [
    cargo
    cargo-cross
    rustup
    yj
    jq
  ] ++ lib.optionals stdenv.isDarwin [
    libiconv
  ];
  # Enable printing backtraces for rust binaries
  RUST_BACKTRACE = 1;
}
