{
  inputs = {
    nix-ros-overlay.url = "github:lopsided98/nix-ros-overlay";
    nixpkgs.follows = "nix-ros-overlay/nixpkgs";  # IMPORTANT!!!
  };
  outputs = { self, nix-ros-overlay, nixpkgs }:
    nix-ros-overlay.inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ nix-ros-overlay.overlays.default ];
        };
      in {
        devShells.default = pkgs.mkShell rec {
          name = "Example project";
          packages = with pkgs.rosPackages.humble; [
            pkgs.colcon
            ros-core
            xacro
            # ...
            # pkgs.libGL
            # pkgs.libxkbcommon
            # pkgs.xorg.libXrandr
            # pkgs.xorg.libX11
            # pkgs.xorg.libXcursor
            # pkgs.xorg.libXrender
            # pkgs.xorg.libXi
          ];
        #   shellHook = ''
        #     export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath packages}
        #   '';
        };
      });
  nixConfig = {
    extra-substituters = [ "https://ros.cachix.org" ];
    extra-trusted-public-keys = [ "ros.cachix.org-1:dSyZxI8geDCJrwgvCOHDoAfOm5sV1wCPjBkKL+38Rvo=" ];
  };
}