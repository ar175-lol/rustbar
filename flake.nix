{
  description = "rustbar - a Wayland status bar for Niri written in Rust";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };
  outputs = inputs @ {
    flake-parts,
    self,
    ...
  }: let
    mkRustbar = pkgs: {
      barWidth ? 1920,
      barHeight ? 30,
      fontName ? "JetBrainsMono Nerd Font",
      fontSize ? 14.0,
      sink ? "@DEFAULT_AUDIO_SINK@",
    }:
      pkgs.rustPlatform.buildRustPackage {
        pname = "rustbar";
        version = "0.1.0";
        src = ./.;
        cargoLock = {lockFile = ./Cargo.lock;};

        nativeBuildInputs = [pkgs.pkg-config pkgs.makeWrapper];
        buildInputs = [
          pkgs.wayland
          pkgs.libxkbcommon
          pkgs.cairo
          pkgs.dbus
        ];

        # only single line consts are patched here
        postPatch = ''
          substituteInPlace src/config.rs \
            --replace 'pub const BAR_WIDTH: u32 = 1920;' 'pub const BAR_WIDTH: u32 = ${toString barWidth};' \
            --replace 'pub const BAR_HEIGHT: u32 = 30;' 'pub const BAR_HEIGHT: u32 = ${toString barHeight};' \
            --replace 'pub const FONT_NAME: &str = "JetBrainsMono Nerd Font";' 'pub const FONT_NAME: &str = "${fontName}";' \
            --replace 'pub const FONT_SIZE: f64 = 14.0;' 'pub const FONT_SIZE: f64 = ${toString fontSize};' \
            --replace 'pub const SINK: &str = "@DEFAULT_AUDIO_SINK@";' 'pub const SINK: &str = "${sink}";'

          substituteInPlace src/main.rs \
            --replace 'layer.set_size(1920, config::BAR_HEIGHT);' 'layer.set_size(${toString barWidth}, config::BAR_HEIGHT);'
        '';

        postFixup = ''
          wrapProgram $out/bin/rustbar \
            --prefix PATH : ${pkgs.lib.makeBinPath [pkgs.wireplumber]}
        '';

        meta = {
          description = "Wayland status bar for Niri";
          mainProgram = "rustbar";
          platforms = pkgs.lib.platforms.linux;
        };
      };
  in
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux"];
      perSystem = {pkgs, ...}: {
        packages.default = mkRustbar pkgs {};
        packages.rustbar = mkRustbar pkgs {};

        devShells.default = pkgs.mkShell {
          inputsFrom = [(mkRustbar pkgs {})];
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
          ];
        };
      };
      flake.homeModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.programs.rustbar;
        package = mkRustbar pkgs {
          inherit (cfg) barWidth barHeight fontName fontSize sink;
        };
      in {
        options.programs.rustbar = {
          enable = lib.mkEnableOption "rustbar Wayland status bar";
          barWidth = lib.mkOption {
            type = lib.types.int;
            default = 1920;
            description = "Bar width in pixels.";
          };
          barHeight = lib.mkOption {
            type = lib.types.int;
            default = 30;
            description = "Bar height in pixels.";
          };
          fontName = lib.mkOption {
            type = lib.types.str;
            default = "JetBrainsMono Nerd Font";
            description = "Font family used for bar text.";
          };
          fontSize = lib.mkOption {
            type = lib.types.float;
            default = 14.0;
            description = "Font size in points.";
          };
          sink = lib.mkOption {
            type = lib.types.str;
            default = "@DEFAULT_AUDIO_SINK@";
            description = "PipeWire/WirePlumber sink name passed to wpctl.";
          };
        };
        config = lib.mkIf cfg.enable {
          home.packages = [package];

          systemd.user.services.rustbar = {
            Unit = {
              Description = "rustbar Wayland status bar";
              Documentation = "https://github.com/ar175-lol/rustbar";
              PartOf = ["graphical-session.target"];
              After = ["graphical-session.target"];
            };
            Service = {
              ExecStart = lib.getExe package;
              RuntimeMaxSec = "1h"; # workaround
              Restart = "always";
              RestartSec = 2;
              TimeoutStopSec = 5;
            };
            Install = {
              WantedBy = ["graphical-session.target"];
            };
          };
        };
      };
    };
}
