{
  description = "BrainBuilder full environment";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      python = pkgs.python311.withPackages (ps: with ps; [
        torch torchvision torchaudio
        numpy pandas
        pyarrow
      ]);
    in {
      devShells.default = pkgs.mkShell {
        buildInputs = [
          pkgs.rustup
          pkgs.racket
          pkgs.clojure
          python
          pkgs.nodejs_20
          pkgs.cargo-tauri
          pkgs.pkg-config
          pkgs.openssl
        ];
        shellHook = ''
          echo "BrainBuilder dev environment loaded."
          export BRAINBUILDER_COMPONENTS_DIR=$PWD/components
        '';
      };
    });
}
