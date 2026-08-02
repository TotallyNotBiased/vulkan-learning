{
  description = "rust flake template";
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      libPath = with pkgs; lib.makeLibraryPath [
        libGL
        libxkbcommon
        wayland
        vulkan-loader
      ];
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rustc
          rustfmt
          clippy
          rust-analyzer

          vulkan-tools
          vulkan-loader
          vulkan-validation-layers
        ];
        shellHook = "echo 'rust env loaded'";
        LD_LIBRARY_PATH = libPath;
      };
    };
}
