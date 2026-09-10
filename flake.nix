{
  description = "brain-map: turn a folder of markdown notes into an interactive knowledge graph";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      lib = nixpkgs.lib;
      forAllSystems = lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      # What the window opens with, and dlopens rather than links: the display protocols,
      # the keyboard map, and GL. Only Linux needs any of it — macOS has its own.
      runtime = pkgs: lib.optionals pkgs.stdenv.hostPlatform.isLinux [
        pkgs.wayland
        pkgs.libxkbcommon
        pkgs.libglvnd
        pkgs.libx11
        pkgs.libxcursor
        pkgs.libxrandr
        pkgs.libxi
      ];
      # glvnd picks a driver by reading a vendor file, and a Nix binary cannot use the
      # host distribution's. Mesa no longer ships one of its own, so this is it: NixOS's
      # `/run/opengl-driver` is still looked at first, and this answers everywhere else.
      eglVendor = pkgs: pkgs.runCommand "brain-map-egl-vendor" { } ''
        mkdir -p $out/share/glvnd/egl_vendor.d
        cat > $out/share/glvnd/egl_vendor.d/50_mesa.json <<EOF
        {"file_format_version":"1.0.0","ICD":{"library_path":"${pkgs.mesa}/lib/libEGL_mesa.so.0"}}
        EOF
      '';
      eglDirs = pkgs:
        "/run/opengl-driver/share/glvnd/egl_vendor.d:${eglVendor pkgs}/share/glvnd/egl_vendor.d";
      # The icons are the colour bitmaps out of this font, read directly rather than
      # drawn by egui — which rasterises outlines and would give grey emoji. Shipping it
      # means the package does not depend on the desktop having installed one, and it is
      # the same Noto build a distribution would.
      emojiFont = pkgs: "${pkgs.noto-fonts-color-emoji}/share/fonts/noto/NotoColorEmoji.ttf";
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          version = (lib.importTOML ./Cargo.toml).package.version;
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "brain-map";
            # One version in the repository, and Cargo.toml holds it. The release tag is
            # checked against the same value, so a bump is a single edit.
            inherit version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
            # The launcher entry and its icon, the same two files the AUR packages install.
            postInstall = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              install -Dm644 packaging/brain-map.desktop \
                $out/share/applications/brain-map.desktop
              install -Dm644 packaging/brain-map.svg \
                $out/share/icons/hicolor/scalable/apps/brain-map.svg
            '';
            buildInputs = runtime pkgs;
            # Nothing above is linked into the binary, so the wrapper is what makes it
            # findable at all.
            postFixup = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              wrapProgram $out/bin/brain-map \
                --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (runtime pkgs)}" \
                --set-default __EGL_VENDOR_LIBRARY_DIRS "${eglDirs pkgs}" \
                --set-default LIBGL_DRIVERS_PATH "${pkgs.mesa}/lib/dri" \
                --set-default BRAIN_MAP_EMOJI_FONT "${emojiFont pkgs}"
            '';
          };
        });

      devShells = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = pkgs.mkShell {
            packages = [
              pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.rust-analyzer pkgs.pkg-config
            ] ++ runtime pkgs;
            # `cargo run` gets what the wrapped package gets.
            shellHook = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              export LD_LIBRARY_PATH="${lib.makeLibraryPath (runtime pkgs)}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
              export __EGL_VENDOR_LIBRARY_DIRS="''${__EGL_VENDOR_LIBRARY_DIRS:-${eglDirs pkgs}}"
              export LIBGL_DRIVERS_PATH="''${LIBGL_DRIVERS_PATH:-${pkgs.mesa}/lib/dri}"
              export BRAIN_MAP_EMOJI_FONT="''${BRAIN_MAP_EMOJI_FONT:-${emojiFont pkgs}}"
            '';
          };
        });
    };
}
