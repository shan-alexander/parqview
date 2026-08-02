{
  description = "parqview — lightweight egui + DuckDB parquet explorer";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          libPath = pkgs.lib.makeLibraryPath [
            pkgs.libx11
            pkgs.libxkbcommon
            pkgs.libGL
            pkgs.libglvnd
            pkgs.wayland
            pkgs.libxcursor
            pkgs.libxi
            pkgs.libxrandr
            pkgs.libxinerama
            pkgs.libxext
            pkgs.libxcb
          ];
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "parqview";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.makeWrapper
            ];
            buildInputs = [
              pkgs.libx11
              pkgs.libxkbcommon
              pkgs.libGL
              pkgs.wayland
              pkgs.duckdb
            ];
            postInstall = ''
              wrapProgram $out/bin/parqview \
                --prefix LD_LIBRARY_PATH : ${libPath} \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.duckdb ]}
            '';
            meta.mainProgram = "parqview";
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          libPath = pkgs.lib.makeLibraryPath [
            pkgs.libx11
            pkgs.libxkbcommon
            pkgs.libGL
            pkgs.libglvnd
            pkgs.wayland
            pkgs.libxcursor
            pkgs.libxi
            pkgs.libxrandr
            pkgs.libxinerama
            pkgs.libxext
            pkgs.libxcb
          ];
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.rustc
              pkgs.cargo
              pkgs.pkg-config
              pkgs.duckdb
              pkgs.libx11
              pkgs.libxkbcommon
              pkgs.libGL
              pkgs.wayland
            ];
            LD_LIBRARY_PATH = libPath;
            shellHook = ''
              echo "parqview shell: LD_LIBRARY_PATH set for Wayland/X11/GL"
              echo "  Wayland default · set PARQVIEW_X11=1 to force X11"
              echo "  cargo run --release -- /path/to/folder_or_file"
            '';
          };
        }
      );
    };
}
