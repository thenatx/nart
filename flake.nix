{
	description = "Flake for Nart (Developing and building)";

	outputs = { self, crane, fenix, flake-utils, ... } @ inputs:
		flake-utils.lib.eachDefaultSystem (system: let
			rust-analyzer = fenix.packages.${system}.stable.rust-analyzer;
      toolchain = with fenix.packages.${system};
          combine [
            minimal.rustc
            minimal.cargo
						complete.rustfmt
						complete.clippy
						complete.rust-src
          ];

			pkgs = inputs.nixpkgs.legacyPackages.${system};
			craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

			cargoArtifacts = craneLib.buildDepsOnly commonArgs;
			commonArgs = {
				src = ./.;
				doCheck = false;
			};

			packageArgs = commonArgs // {
				cargoArtifacts = cargoArtifacts;
			};
			buildInputs = with pkgs; [
        wayland
        wayland.dev
        libxkbcommon
        libxkbcommon.dev
        pkg-config

				libGL
		    vulkan-headers vulkan-loader
    		vulkan-tools vulkan-tools-lunarg
    		vulkan-extension-layer
    		vulkan-validation-layers
		];

		in {
			packages = {
				default = craneLib.buildPackage packageArgs // {
					CARGO_LINKER = "clang";
    			CARGO_RUSTFLAGS = "-Clink-arg=-fuse-ld=${pkgs.mold}/bin/mold";
				};
			};

			checks = {
				clippy = craneLib.cargoClippy packageArgs;
				fmt = craneLib.cargoFmt packageArgs;
			};

			devShells.default = craneLib.devShell {
				checks = self.checks.${system};
				packages = [ rust-analyzer ] ++ buildInputs;
				LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
			};
	});

	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
		flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
		fenix.url = "github:nix-community/fenix/monthly";
	};
}
