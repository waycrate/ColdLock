{pkgs}:
pkgs.mkShell {
  name = "ColdLock shell";
  nativeBuildInputs = with pkgs; [
    # Compilers
    cargo
    rustc
    scdoc
    rustPlatform.bindgenHook

    # Deps
    libxkbcommon

    wayland
    libGL
    vulkan-loader
    linux-pam
    libclang
    pkg-config

    # Tools
    cargo-audit
    cargo-deny
    clippy
    rust-analyzer
    rustfmt
  ];

  buildInputs = with pkgs; [
    linux-pam
    llvmPackages.libclang
    clang
  ];

  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    wayland
    libxkbcommon
    libGL
    vulkan-loader
    fontconfig
    freetype
  ]);
}
