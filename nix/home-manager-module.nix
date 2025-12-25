{ config, lib, osConfig, pkgs, ... }:
let
  inherit (lib) mkEnableOption mkIf;
  cfg = config.services.asker-prompt;
in
{
  options = {
    services.asker-prompt = {
      enable = mkEnableOption "asker-prompt";
    };
  };

  config = mkIf cfg.enable {
    systemd.user.services.asker-prompt = {
      Service = {
        Environment = [
          "ASKER_DIR=${osConfig.services.asker.runtimeDir}"
        ];
        ExecStart = "${pkgs.asker-prompt}/bin/asker-prompt";
      };
      Install = {
        WantedBy = ["graphical-session.target"];
      };
    };
  };
}
