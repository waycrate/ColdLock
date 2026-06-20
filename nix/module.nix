self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.coldlock;
  inherit (pkgs.stdenv.hostPlatform) system;
  inherit (lib) types;
  inherit (lib.modules) mkIf;
  inherit (lib.options) mkOption mkEnableOption;
in {
  options.programs.coldlock = {
    enable = mkEnableOption "ColdLock";

    package = mkOption {
      description = "The package to use for `coldlock`";
      default = self.packages.${system}.default;
      type = types.package;
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [cfg.package];

    security.pam.services.system-auth = {};
  };
}
