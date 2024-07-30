{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) ];
        };

        buildInputs = with pkgs; [ rust-bin.stable.latest.default ];
        darwinBuildInputs = with pkgs;
          with pkgs.darwin;
          with pkgs.darwin.apple_sdk.frameworks; [
            libiconv
            Security
            AppKit
            CoreFoundation
            CoreAudio
            AudioToolbox
            AudioUnit
          ];

      in {
        formatter = pkgs.nixpkgs-fmt;
        devShells.default =
          pkgs.mkShell { buildInputs = buildInputs ++ darwinBuildInputs; };
      });
}

