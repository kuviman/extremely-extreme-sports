{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-stable.url = "nixpkgs/release-23.11";
    systems.url = "github:nix-systems/default";
    geng.url = "github:geng-engine/cargo-geng";
    geng.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs = { geng, nixpkgs, nixpkgs-stable, systems, self }:
    let
      pkgsFor = system: import nixpkgs { inherit system; };
      forEachSystem = f: nixpkgs.lib.genAttrs (import systems) (system:
        let
          pkgs = pkgsFor system;
          pkgs-stable = import nixpkgs-stable { inherit system; };
        in
        f { inherit system pkgs pkgs-stable; });
    in
    {
      devShells = forEachSystem ({ system, pkgs, pkgs-stable, ... }:
        {
          default = geng.lib.mkShell {
            inherit system;
            target.linux.enable = true;
            target.web.enable = true;
            packages = with pkgs; [
              just
              (pkgs-stable).butler
            ];
          };
        });
      formatter = forEachSystem ({ pkgs, ... }: pkgs.nixpkgs-fmt);
    };
}
