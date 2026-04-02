{
  description = "IsoNim-Freya — Nim bindings for the Freya native GUI framework";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        fenixPkgs = fenix.packages.${system};
        rustToolchain = fenixPkgs.stable.withComponents [
          "cargo"
          "clippy"
          "rustc"
          "rust-src"
          "rust-std"
          "rustfmt"
        ];
        isLinux = pkgs.lib.hasSuffix "linux" system;
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            [
              rustToolchain
              pkgs.nim
              pkgs.nimble
              pkgs.just
              pkgs.pkg-config
              pkgs.openssl
              pkgs.cmake
            ]
            ++ pkgs.lib.optionals isLinux [
              # Skia / GPU dependencies
              pkgs.fontconfig
              pkgs.freetype
              pkgs.libGL
              pkgs.libxkbcommon
              pkgs.vulkan-loader

              # Wayland
              pkgs.wayland
              pkgs.wayland-protocols

              # X11
              pkgs.libx11
              pkgs.libxcursor
              pkgs.libxi
              pkgs.libxrandr
              pkgs.libxcb

              # Headless GUI testing
              pkgs.xorg.xorgserver   # Xvfb
              pkgs.xorg.xdpyinfo     # X11 display info (Xvfb readiness check)
              pkgs.mesa               # Software GL (llvmpipe)
              pkgs.libglvnd           # GL dispatch
              pkgs.sway               # Wayland compositor (headless mode)
              pkgs.weston             # Wayland compositor (lightweight headless)
              pkgs.wayland-utils      # wayland-info
              pkgs.wf-recorder        # Wayland screen recorder
              pkgs.ffmpeg-full        # Video recording/encoding
              pkgs.mpv                # Video player for streaming
            ];

          # Ensure the linker can find native libs at build time
          LD_LIBRARY_PATH = pkgs.lib.optionalString isLinux (
            pkgs.lib.makeLibraryPath [
              pkgs.fontconfig
              pkgs.freetype
              pkgs.libGL
              pkgs.libxkbcommon
              pkgs.vulkan-loader
              pkgs.wayland
              pkgs.libx11
              pkgs.libxcursor
              pkgs.libxi
              pkgs.libxrandr
              pkgs.libxcb
              pkgs.mesa
              pkgs.libglvnd
              pkgs.stdenv.cc.cc.lib
            ]
          );

          shellHook = ''
            echo "isonim-freya dev shell — rust $(rustc --version), nim $(nim --version 2>&1 | head -1)"
          '';
        };
      }
    );
}
