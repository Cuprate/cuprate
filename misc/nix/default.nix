{ pkgs ? import <nixpkgs> {}, rev }:

let
  sourceRoot = ../..;

  manifest = builtins.fromTOML (
    builtins.readFile (
      sourceRoot + "/binaries/cuprated/Cargo.toml"
    )
  );

in
pkgs.rustPlatform.buildRustPackage {
  pname = "cuprate";
  version = manifest.package.version;

  GITHUB_SHA = rev;

  src = pkgs.lib.cleanSourceWith {
    src = sourceRoot;
    filter = path: type:
      builtins.baseNameOf path != "target"
      && pkgs.lib.cleanSourceFilter path type;
  };

#  cargoHash = pkgs.lib.fakeHash;
  cargoHash = "sha256-amPnBlziVSChGr4kt0nry6UpfN/HWbnYYwbPKuKGCUM=";

  nativeBuildInputs = [
    pkgs.cmake
    pkgs.git
    pkgs.patchelf
    pkgs.pkg-config
  ];

  buildInputs = [
    pkgs.stdenv.cc.cc.lib
  ];

  dontUseCmakeConfigure = true;

  cargoBuildFlags = [
    "--package"
    "cuprated"
    "--bin"
    "cuprated"
  ];

  doCheck = false;

  postFixup = ''
    patchelf --add-rpath \
      "${pkgs.lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ]}" \
      "$out/bin/cuprated"
  '';

  meta = {
    description = "Cuprate Monero node";
    mainProgram = "cuprated";
    license = pkgs.lib.licenses.agpl3Only;
    platforms = pkgs.lib.platforms.linux;
  };
}
