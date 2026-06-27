{
  description =
    "Zakhor — MCP server for persistent knowledge graph memory backed by GNOME Tracker SPARQL";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    crane = {
      url = "github:ipetkov/crane";
      # NOTE: newer Crane versions no longer have a nixpkgs input
    };

    pyproject-nix = {
      url = "github:pyproject-nix/pyproject.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    uv2nix = {
      url = "github:pyproject-nix/uv2nix";
      inputs.pyproject-nix.follows = "pyproject-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, pyproject-nix, uv2nix }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        # ──────────────────────────────────────────────
        # Rust toolchain & Crane
        # ──────────────────────────────────────────────
        craneLib = crane.mkLib pkgs;

        # Pre-fetch swagger-ui for utoipa-swagger-ui build.rs (no network in sandbox)
        swagger-ui-zip = pkgs.fetchurl {
          url =
            "https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.32.6.zip";
          hash = "sha256-s8B+CRVZtZqDP2ZUfrH8GPKJb5bj8flT4vHvsyiqM5Q=";
        };

        # Common arguments shared across Crane derivations
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          # Native deps: tracker-sparql-3.0 (tinysparql), glib (gio), openssl, pkg-config
          buildInputs = with pkgs; [ glib tinysparql openssl ];
          nativeBuildInputs = with pkgs; [ pkg-config curl ];
          # Pre-copy swagger-ui zip into source so build.rs can read it
          # (Nix sandbox can't read arbitrary store paths via file:// in build.rs)
          preBuild = ''
            cp "${swagger-ui-zip}" swagger-ui.zip
            chmod 644 swagger-ui.zip
            export SWAGGER_UI_DOWNLOAD_URL="file://$(pwd)/swagger-ui.zip"
          '';
          # Crane auto-detects Cargo.lock from the cleaned source
        };

        # Build dependencies separately so the main package shares them
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # ── Rust package ─────────────────────────────
        zakhor =
          craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });

        # ──────────────────────────────────────────────
        # Frontend (TypeScript / pnpm)
        # ──────────────────────────────────────────────
        frontendNode = pkgs.nodejs_22;
        frontendPnpm = pkgs.pnpm_11;

        frontendPnpmDeps = pkgs.fetchPnpmDeps {
          pnpm = frontendPnpm;
          src = ./ui;
          pname = "zakhor-frontend";
          fetcherVersion = 3;
          hash = "sha256-/m7Y97sZSo5fXeL4Z5aimvdT/+T19B7twFylKxp+L8I=";
        };

        zakhor-frontend = pkgs.stdenv.mkDerivation {
          pname = "zakhor-frontend";
          version = "0.1.0";
          src = ./ui;

          nativeBuildInputs = [ frontendNode frontendPnpm pkgs.pnpmConfigHook ];

          pnpmDeps = frontendPnpmDeps;

          buildPhase = ''
            runHook preBuild
            pnpm run generate:routes
            pnpm run build
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            mkdir -p $out
            cp -r dist/* $out/
            runHook postInstall
          '';

          # Vite/TypeScript generates a lot of GC roots; suppress warnings
          env.NIX_BUILD_CORES = 0;
        };

        # Helper for lightweight frontend checks
        frontend-check = checkName: buildPhaseScript:
          pkgs.runCommand checkName {
            nativeBuildInputs = [ frontendNode frontendPnpm ];
            pnpmDeps = frontendPnpmDeps;
            src = ./ui;
            HOME = "$TMPDIR/home";
          } ''
            mkdir -p "$HOME"
            cd "$src"
            ${buildPhaseScript}
            touch "$out"
          '';

        # ──────────────────────────────────────────────
        # Python test environment (uv2nix)
        # ──────────────────────────────────────────────
        workspace = uv2nix.lib.${system}.workspace.loadWorkspace {
          workspaceRoot = ./tests/python;
        };

        overlay = workspace.mkPyprojectOverlay { sourcePreference = "source"; };

        pythonSet = (pkgs.callPackage pyproject-nix.build.packages {
          python = pkgs.python312;
        }).overrideScope overlay;

        zakhor-python-tests = pythonSet.mkVirtualEnv "zakhor-python-tests" {
          zakhor-integration-tests = [ ];
        };

      in {
        # ── Packages ───────────────────────────────────
        packages = {
          inherit zakhor zakhor-frontend zakhor-python-tests;
          default = zakhor;
        };

        # ── Checks ─────────────────────────────────────
        checks = {
          rust-fmt = craneLib.cargoFmt { src = craneLib.cleanCargoSource ./.; };

          rust-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-- --deny warnings";
          });

          rust-test =
            craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });

          frontend-lint = frontend-check "frontend-lint" ''
            pnpm run lint
          '';

          frontend-typecheck = frontend-check "frontend-typecheck" ''
            pnpm run typecheck
          '';

          frontend-test = frontend-check "frontend-test" ''
            pnpm run test
          '';

          frontend-build = zakhor-frontend;

          python-test = pkgs.runCommand "python-test" {
            buildInputs = [ zakhor-python-tests ];
            # Pass through from CI env; integration test requires running tracker endpoint
            TRACKER_ENDPOINT = null;
          } ''
            export TRACKER_ENDPOINT=''${TRACKER_ENDPOINT:-}
            if [ -z "$TRACKER_ENDPOINT" ]; then
              echo "SKIP: TRACKER_ENDPOINT not set — integration test requires running tracker endpoint"
              touch "$out"
              exit 0
            fi
            cd ${./tests/python}
            ${zakhor-python-tests}/bin/pytest -v
            touch "$out"
          '';
        };

        # ── DevShell ──────────────────────────────────
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # Rust toolchain
            cargo
            rustc
            rustfmt
            clippy
            # Node.js / pnpm
            nodejs_22
            pnpm_11
            # Python
            python312
            uv
            # Native build deps
            pkg-config
            glib
            tinysparql
            openssl
            # System build tools
            gcc
          ];

          env = { TRACKER_ENDPOINT = "http://127.0.0.1:7878"; };
        };
      });
}
