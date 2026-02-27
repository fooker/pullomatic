{ lib
, rustPlatform
, pkg-config
, openssl
, libgit2
,
}:

rustPlatform.buildRustPackage {
  pname = "pullomatic";
  version = "0.2.0";

  src = lib.cleanSource ./.;

  uesFetchCargoVendor = true;
  cargoHash = "sha256-0rumDPfeZRrHe69TZTBQutJKHDZkfaYXYZSFG7CywfU=";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
    libgit2
  ];

  LIBGIT2_NO_VENDOR = 1;

  meta = with lib; {
    description = "Automated git pulls";
    mainProgram = "pullomatic";
    homepage = "https://github.com/fooker/pullomatic";
    license = licenses.mit;
    maintainers = with maintainers; [ fooker ];
  };
}

