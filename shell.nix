let
  pinned = import ./pinned.nix;
in
{ pkgs ? pinned }:

pkgs.mkShell {

  buildInputs = with pkgs; [
    cargo
    rls
    yj
    jq
  ] ++ lib.optionals stdenv.isDarwin [
    libiconv
  ] ++ lib.optionals ((builtins.getEnv "GITHUB_WORKFLOW") != "") [
    cargo-cross
    rustup
  ];
  # Enable printing backtraces for rust binaries
  RUST_BACKTRACE = 1;
}
