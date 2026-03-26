{
  pkgs,
  ...
}:
{
  imports = [
    {
      packages = [
        # NATS
        pkgs.natscli
      ];
      services.nats = {
        enable = true;
        jetstream.enable = true;
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
