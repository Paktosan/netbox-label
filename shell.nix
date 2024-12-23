with import <nixpkgs> {};
mkShell {
  nativeBuildInputs= [
    rustup
    gcc
    pkg-config
    openssl
  ];
}
