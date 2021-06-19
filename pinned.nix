let
  unstableTgz = builtins.fetchTarball {
    # Descriptive name to make the store path easier to identify
    name = "nixos-nixos-21.05-2021-06-06";
    # Be sure to update the above if you update the archive
    url = https://github.com/nixos/nixpkgs/archive/93963c27b934f24289a94b9e3784d60a9b77e92c.tar.gz;
    sha256 = "1awvfa7nrk3fdl48q75qa1rsc8hq22xmaqrn4qzd2ncr8s9kgpfd";
  };
in
import unstableTgz {}
