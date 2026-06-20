{
  lib,
  rustPlatform,
  pkg-config,
  autoPatchelfHook,
  wayland,
  libxkbcommon,
  libGL,
  vulkan-loader,
  linux-pam,
  libclang,
}:
rustPlatform.buildRustPackage rec {
  pname = "coldlock";
  version = "${(builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"))).package.version}-git";

  src = lib.cleanSource ./..;

  cargoLock.lockFile = "${src}/Cargo.lock";

  nativeBuildInputs = [
    pkg-config
    autoPatchelfHook
    rustPlatform.bindgenHook
  ];

  runtimeDependencies = [
    wayland
    libxkbcommon
    libGL
    vulkan-loader
  ];

  buildInputs =
    [
      linux-pam
      libclang
    ]
    ++ runtimeDependencies;
}
