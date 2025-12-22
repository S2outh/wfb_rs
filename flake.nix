{
  description = "rust flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };
      in
      {
        devShells.default =
        let
          rust = pkgs.fenix.stable.toolchain;
        in
        pkgs.mkShell {
          buildInputs = with pkgs; [
            rust
            
            libpcap
            iw
          ];

					# set default defmt log level
					DEFMT_LOG = "info";
        };
      }
    );
}
