{
  pkgs,
  lib,
  ...
}:
{
  imports = [
    {
      packages = [
        # NATS
        pkgs.natscli
      ];
      processes.nats-server = {
        exec = "${lib.getExe pkgs.nats-server} -js -DV -sd .devenv/state/nats";
        process-compose = {
          ready_log_line = "Server is ready";
        };
      };
    }
  ];
  config = {
    languages.rust = {
      channel = "stable";
      enable = true;
    };
    git-hooks.enable = false;
    git-hooks.hooks = {
      rustfmt = {
        enable = false;
        files = "\.rs$";
      };
      clippy = {
        enable = false;
        settings = {
          allFeatures = true;
        };
      };
    };
  };
}
