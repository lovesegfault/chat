{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:figsoda/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.naersk.follows = "naersk";
    };
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, fenix, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenixPkgs = fenix.packages.${system};
        fenixComplete = fenixPkgs.complete.withComponents [
          "cargo"
          "clippy-preview"
          "llvm-tools-preview"
          "rust-src"
          "rust-std"
          "rustc"
          "rustfmt-preview"
        ];
        naerskBuild = (naersk.lib.${system}.override {
          cargo = fenixComplete;
          rustc = fenixComplete;
        }).buildPackage;
      in
      {
        defaultPackage = naerskBuild {
          src = ./.;
          doCheck = true;
        };

        devShell = self.defaultPackage.${system}.overrideAttrs (oldAttrs: {
          buildInputs = with pkgs; (oldAttrs.buildInputs or [ ]) ++ [
            cargo-edit
            cargo-udeps
            cargo-watch
            fenixPkgs.rust-analyzer
            linuxPackages_latest.perf
            nixpkgs-fmt
            telnet
          ];
        });
      });
}
