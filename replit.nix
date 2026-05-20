{ pkgs }: {
  deps = [
    pkgs.bash
    pkgs.curl
    pkgs.python312
    pkgs.python312Packages.pip
    pkgs.python312Packages.requests
  ];
}
