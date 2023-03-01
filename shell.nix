{ pkgs ? import <nixpkgs> {}}:

let
  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  ruststable = (pkgs.latest.rustChannels.stable.default.override {
      extensions = [
        "rust-src"
      ];
      targets = [ "x86_64-unknown-linux-musl" ];
    });
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    ruststable
    sqlx-cli
    protobuf
    trunk
    cargo-tauri
    pkgconfig
    webkitgtk
    cairo
    libsoup
    wasm-bindgen-cli
    openssl
    wget
  ];
  
  RUST_BACKTRACE = 1;
  WEBKIT_DISABLE_COMPOSITING_MODE=1;
}
