{
  description = "brain-map: turn a folder of markdown notes into an interactive knowledge graph";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      # The webview stack wry links against. Only Linux needs it: macOS uses WKWebView,
      # which comes with the system.
      webview = pkgs: nixpkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
        pkgs.webkitgtk_4_1   # pkg-config webkit2gtk-4.1 and javascriptcoregtk-4.1
        pkgs.gtk3
        pkgs.libsoup_3
        pkgs.glib
      ];
    in
    {
      packages = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "brain-map";
            # One version in the repository, and Cargo.toml holds it. The release tag
            # is checked against the same value, so a bump is a single edit.
            version = (nixpkgs.lib.importTOML ./Cargo.toml).package.version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config ]
              ++ nixpkgs.lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.wrapGAppsHook3;
            buildInputs = webview pkgs;
            # WebKitGTK aborts outright without an EGL display, and a Nix binary cannot
            # load the host distribution's driver. Point it at Nix's own Mesa unless the
            # environment already names a working one.
            preFixup = nixpkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              gappsWrapperArgs+=(
                --set-default __EGL_VENDOR_LIBRARY_DIRS "/run/opengl-driver/share/glvnd/egl_vendor.d:${pkgs.mesa}/share/glvnd/egl_vendor.d"
              )
            '';
          };
        });

      devShells = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.rust-analyzer pkgs.pkg-config ]
              ++ webview pkgs;
            # `cargo run` gets the same EGL fallback the wrapped package gets.
            shellHook = nixpkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              export __EGL_VENDOR_LIBRARY_DIRS="''${__EGL_VENDOR_LIBRARY_DIRS:-/run/opengl-driver/share/glvnd/egl_vendor.d:${pkgs.mesa}/share/glvnd/egl_vendor.d}"
            '';
          };
        });
    };
}
