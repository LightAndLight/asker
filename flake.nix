{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay?rev=6b5c52313aaf3f3e1a0a6757bb89846edfb5195c";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix";
      inputs.flake-utils.follows = "flake-utils";
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, cargo2nix  }:
    let
      rustVersion = "1.92.0";

      packageOverrides =
        pkgs:
        with pkgs.rustBuilder.rustLib;
        pkgs.rustBuilder.overrides.all ++
        [
          (makeOverride {
            name = "glib-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.glib
              ];
            };
          })
        
          (makeOverride {
            name = "cairo-sys-rs";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.cairo
              ];
            };
          })
        
          (makeOverride {
            name = "gobject-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.glib
              ];
            };
          })
        
          (makeOverride {
            name = "graphene-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.glib
                pkgs.graphene
              ];
            };
          })
        
          (makeOverride {
            name = "gio-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.glib
              ];
            };
          })
        
          (makeOverride {
            name = "pango-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.pango
              ];
            };
          })
        
          (makeOverride {
            name = "gdk-pixbuf-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.gdk-pixbuf
              ];
            };
          })
        
          (makeOverride {
            name = "gdk4-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.gtk4
              ];
            };
          })
        
          (makeOverride {
            name = "gsk4-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.gtk4
              ];
            };
          })
        
          (makeOverride {
            name = "gtk4-sys";
            overrideAttrs = drv: {
              buildInputs = (drv.buildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.gtk4
              ];
            };
          })
        ];

      overlays.default =
        final: prev:
        let
          withCargo2nix = prev.extend cargo2nix.overlays.default;
          withRustOverlay = withCargo2nix.extend rust-overlay.overlays.default;

          pkgs = withRustOverlay;
          rustPkgs = pkgs.rustBuilder.makePackageSet {
            inherit packageOverrides rustVersion;
            packageFun = import ./Cargo.nix;
          };
        in
          {
            asker = rustPkgs.workspace.asker {};
            asker-prompt = rustPkgs.workspace.asker-prompt {};
          };
    in
    {
      inherit overlays;
      nixosModules = {
        nixos = import ./nix/nixos-module.nix;
        home-manager = import ./nix/home-manager-module.nix;
      };
    } //
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            cargo2nix.overlays.default
            rust-overlay.overlays.default
            overlays.default
          ];
        };

        packages = {
          asker = pkgs.asker;
          asker-prompt = pkgs.asker-prompt;
        };
      in {
        inherit packages;
        devShell = pkgs.mkShell {
          buildInputs = [
            (pkgs.rust-bin.stable.${rustVersion}.default.override {
              extensions = [
                "cargo"
                "clippy"
                "rustc"
                "rust-src"
                "rustfmt"
                "rust-analyzer"
              ];
            })
            cargo2nix.packages.${system}.cargo2nix

            pkgs.pkg-config
            pkgs.glib
            pkgs.cairo
            pkgs.gtk4
          ];
        };
      }
    );
}
