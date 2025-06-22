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
  cargoHash = "sha256-QHjuTn5je5l4F10nqP1I4PuVhb1o059UltNgfM2R8dY=";

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

