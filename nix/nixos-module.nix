{ config, lib, pkgs, ... }:
let
  inherit (lib)
    mkEnableOption
    mkIf
    mkPackageOption
    mkOption
    types
  ;

  cfg = config.services.asker;
in
{
  options = {
    services.asker = {
      enable = mkEnableOption "asker, a system for requesting user input";
      user = mkOption {
        description = "The `asker` user";
        default = "asker";
        readOnly = true;
      };
      group = mkOption {
        description = "The `asker` group";
        default = "asker";
        readOnly = true;
      };
      runtimeDir = mkOption {
        description = "`asker`'s runtime directory";
        default = "/run/asker";
        readOnly = true;
      };
      keys = mkOption {
        description = "Inputs that can be requested";
        type = types.attrsOf (types.submodule
          ({ name, ... }: {
            options = {
              description = mkOption {
                description = "The key's description";
                type = types.str;
                example = "your KeepassXC password";
              };
              group = mkOption {
                description = "The group name associated with the key";
                type = types.str;
                default = "asker-key-${name}";
                readOnly = true;
              };
            };
          }));
        example = {
          password-a = {
            description = "password A";
          };
          password-b = {
            description = "password B";
          };
        };
      };
    };
  };

  config =
  let
    keyGroups =
      builtins.listToAttrs
        (builtins.map
          ({ name, value }: { name = "asker-key-${name}"; value = {}; })
          (lib.attrsToList cfg.keys))
    ;
  in
  mkIf cfg.enable {
    users.groups = { "${cfg.group}" = {}; asker-keys = {}; } // keyGroups;

    users.users."${cfg.user}" = {
      isSystemUser = true;
      group = cfg.group;
      extraGroups = ["asker-keys"] ++ lib.attrNames keyGroups;
    };

    systemd.tmpfiles.settings.asker =
      {
        "${cfg.runtimeDir}" = {
          d = {
            mode = "0755";
            user = "asker";
            group = "asker-keys";
          };
        };
      } //
      builtins.listToAttrs
        (builtins.concatMap
          ({ name, value }:
            [
              {
                name = "${cfg.runtimeDir}/${name}";
                value = {
                  d = {
                    mode = "0775";
                    user = "asker";
                    group = "asker-key-${name}";
                  };
                };
              }
              {
                name = "${cfg.runtimeDir}/${name}/description";
                value = {
                  "L+" = {
                    argument = "${pkgs.writeText "asker-key-${name}-description" value.description}";
                  };
                };
              }
              {
                name = "${cfg.runtimeDir}/${name}/garbage";
                value = {
                  f = {
                    # u:rw,g:rw,o:rw,
                    mode = "0666";
                    user = "asker";
                    group = "asker-key-${name}";
                  };
                };
              }
            ]
          )
          (lib.attrsToList cfg.keys));
  };
}
