# diaria Arch binary repository

Binary pacman repository for
[diaria](https://github.com/corrodedHash/diaria-rs), an encrypted plaintext
command-line diary. Packages are rebuilt from the upstream `main` branch by CI
and pushed here as `.pkg.tar.zst` artifacts alongside the `repo-add`-generated
`diaria.db` / `diaria.files` databases.

## Install

```ini
# /etc/pacman.conf
[diaria]
Server = https://raw.githubusercontent.com/<owner>/<this-repo>/main/
```

Replace `<owner>/<this-repo>` with this repository's full path (e.g.
`corrodedHash/diaria-arch`). Then:

```sh
sudo pacman -Syu
sudo pacman -S diaria-rs-git
```

`pacman` will fetch the current binary package and install
`/usr/bin/diaria`, documentation, and the MIT license. The package is
`x86_64`-only by default (see _Architecture support_ below).

## Updates

A GitHub Actions workflow in the upstream diaria-rs repo builds the package
inside an `archlinux:latest` container every time `main` moves, runs the
upstream test suite (`check()`) before packaging, then pushes the new
`.pkg.tar.zst` here and regenerates `diaria.db` / `diaria.files` via
`repo-add`. The workflow also removes the previous package file from this
repository so only the latest version remains on disk and in the database;
older binaries are still available in this repo's git history if you ever need
to roll back.

## Architecture support

- `x86_64`: prebuilt binary provided here.
- `aarch64`: no prebuilt binary. Use the [`PKGBUILD` in the upstream
  repo](https://github.com/corrodedHash/diaria-rs/blob/main/dist/pacman/PKGBUILD)
  directly: clone diaria-rs, `cd dist/pacman`, `makepkg -si` on the aarch64
  host. The `PKGBUILD` declares `arch=('x86_64' 'aarch64')` so makepkg accepts
  the build.

## Manual build / rollback

The `PKGBUILD` that produced the binary in this repo lives in the upstream
[diaria-rs repository](https://github.com/corrodedHash/diaria-rs) under
`dist/pacman/`. To build a binary yourself, by hand, against the diaria-rs
`HEAD`:

```sh
git clone https://github.com/corrodedHash/diaria-rs.git
cd diaria-rs/dist/pacman
makepkg -si
```

To rollback to an older binary in case of regression, clone this repo,
check out the commit that contains the binary you want, copy the `.pkg.tar.zst`
aside, then `sudo pacman -U path/to/that.pkg.tar.zst`.