{
  description = "Simple flake.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { nixpkgs, crane, self } : let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
    craneLib = crane.mkLib pkgs;
  in {
    # nix develop
    devShells.${system}.default = craneLib.devShell {
      buildInputs = with pkgs; [
        just
        ffmpeg
        pkg-config
        libgbm
        libGL
        libglvnd
        glib
        gtk2
        gst_all_1.gstreamer
        gst_all_1.gst-plugins-ugly
        gst_all_1.gst-plugins-bad
        gst_all_1.gst-plugins-base
        gst_all_1.gst-plugins-good
        gst_all_1.gst-plugins-rs
        gst_all_1.gst-libav
        gst_all_1.gst-vaapi
      ];

      buildInputsNative = with pkgs; [
        pkg-config
      ];

      LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath (with pkgs; [
        wlroots
        libclang
        libGL
        libglvnd
      ])}";
    };
  };
}
