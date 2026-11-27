{
  description = "DimaSter: GPU-accelerated terminal emulator with Lua configuration";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      each = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      package = pkgs:
        let
          # winit and wgpu load these at run time
          runtime = with pkgs; [
            fontconfig libxkbcommon wayland vulkan-loader libGL
            xorg.libX11 xorg.libXcursor xorg.libXi xorg.libxcb
          ];
        in
        pkgs.rustPlatform.buildRustPackage {
          pname = "dimaster";
          version = (nixpkgs.lib.importTOML ./Cargo.toml).workspace.package.version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "-p" "dimaster" ];
          nativeBuildInputs = with pkgs; [ pkg-config patchelf ];
          buildInputs = runtime;
          # tests need system fonts
          doCheck = false;
          # rpath instead of LD_LIBRARY_PATH, which would leak into the shell
          postFixup = ''
            patchelf --add-rpath ${nixpkgs.lib.makeLibraryPath runtime} $out/bin/dimaster
          '';
          postInstall = ''
            install -Dm644 assets/dimaster.svg $out/share/icons/hicolor/scalable/apps/dimaster.svg
            install -Dm644 assets/dimaster.png $out/share/icons/hicolor/128x128/apps/dimaster.png
            install -Dm644 packaging/flatpak/io.github.Utkautka1.DimaSter.desktop \
              $out/share/applications/dimaster.desktop
            substituteInPlace $out/share/applications/dimaster.desktop \
              --replace-fail "Exec=dimaster-flatpak" "Exec=dimaster" \
              --replace-fail "Icon=io.github.Utkautka1.DimaSter" "Icon=dimaster"
          '';
          meta.mainProgram = "dimaster";
        };
    in
    {
      packages = each (pkgs: { default = package pkgs; });
      apps = each (pkgs: {
        default = { type = "app"; program = "${package pkgs}/bin/dimaster"; };
      });
    };
}
