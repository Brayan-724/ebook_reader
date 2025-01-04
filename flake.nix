{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = {nixpkgs, ...}: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in {
    devShells.${system}.default = pkgs.mkShell {
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
        gst_all_1.gst-plugins-base
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
