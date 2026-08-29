{
  description = "brain-map: turn a folder of markdown notes into an interactive knowledge graph";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
    in
    {
      packages = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "brain-map";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
          };
        });

      devShells = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.rust-analyzer ];
          };
        });
    };
}
