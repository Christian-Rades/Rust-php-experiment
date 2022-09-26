{pkgs ? (import <unstable>) {}}:
with pkgs; let
  phpunwrapped = php81.unwrapped.dev.overrideAttrs (attrs: {
    configureFlags = attrs.configureFlags ++ ["--enable-zts"];
    preConfigure = ''
      for i in main/build-defs.h.in scripts/php-config.in; do
               substituteInPlace $i \
                 --replace '@CONFIGURE_COMMAND@' '(omitted)' \
                 --replace '@PHP_LDFLAGS@' ""
             done
             export EXTENSION_DIR=$out/lib/php/extensions
             for i in $(find . -type f -name "*.m4"); do
               substituteInPlace $i \
                 --replace 'test -x "$PKG_CONFIG"' 'type -P "$PKG_CONFIG" >/dev/null'
             done
             ./buildconf --copy --force
             if test -f $src/genfiles; then
               ./genfiles
             fi
    '';
  });
  php = phpunwrapped.buildEnv {
    extensions = {
      enabled,
      all,
    }:
      enabled
      ++ (with all; [
        redis
        pcov
      ]);
    extraConfig = "memory_limit = -1";
  };
in
  rustPlatform.buildRustPackage rec {
    pname = "rust-ext";
    version = "0.0.1";

    # Needed so bindgen can find libclang.so
    LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";

    nativeBuildInputs = [rust-analyzer rustfmt clippy openssl openssl.dev pkg-config php phpunwrapped clang stdenv.cc.libc];
    buildInputs = [
      rustfmt
      clippy

      openssl
      php
      phpunwrapped
      clang
      stdenv.cc.libc
    ];

    src = ./.;

    cargoSha256 = "0kmk8qw12zdz0bx1nl3rydlyf6vn8k8n5zyxgcf6ix92r3ldsb6f";
  }
