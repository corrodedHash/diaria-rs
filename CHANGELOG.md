# Changelog

## [1.3.0](https://github.com/corrodedHash/diaria-rs/compare/v1.2.1...v1.3.0) (2026-07-21)


### Features

* add build attestation to release workflow ([#24](https://github.com/corrodedHash/diaria-rs/issues/24)) ([d7da561](https://github.com/corrodedHash/diaria-rs/commit/d7da5614d1f811e1d57651eedb8bdc91ea38f249))

## [1.2.1](https://github.com/corrodedHash/diaria-rs/compare/v1.2.0...v1.2.1) (2026-07-20)


### Bug Fixes

* strip full path and extension from entry selector display ([#22](https://github.com/corrodedHash/diaria-rs/issues/22)) ([287dd4e](https://github.com/corrodedHash/diaria-rs/commit/287dd4e08c280233907b96aa43d94c4c3ebfd229))

## [1.2.0](https://github.com/corrodedHash/diaria-rs/compare/v1.1.1...v1.2.0) (2026-07-19)


### Features

* add free-tier CI release pipeline ([9539e06](https://github.com/corrodedHash/diaria-rs/commit/9539e061e00eaf73f6bc9e9ef8ec93923c988a25))
* compression ([86d9bec](https://github.com/corrodedHash/diaria-rs/commit/86d9becf225e0ed0427bf3c2db19c339dc635c14))
* implement custom pacman repo push ([d12cdfb](https://github.com/corrodedHash/diaria-rs/commit/d12cdfbf8b328e60fceabdd53d46e793b4ca0b9d))
* implement docker container sandboxed agent ([1b21765](https://github.com/corrodedHash/diaria-rs/commit/1b2176523cda8f3c0b082174440617f63bfcc8fc))
* initial encryption / encoding ([31c1de9](https://github.com/corrodedHash/diaria-rs/commit/31c1de97579120f2754f076c4a8f72742d0cbe02))
* initial implementation of all subcommands ([893011c](https://github.com/corrodedHash/diaria-rs/commit/893011ca235330b92a88e1bf7b3e2217f65af27b))
* inject UserOutput into DiariaFsRepository for warnings ([4a274ca](https://github.com/corrodedHash/diaria-rs/commit/4a274ca6c91250c2681a712e80400f0b0867e5ca))
* made private key encryption less weird ([a7fdf96](https://github.com/corrodedHash/diaria-rs/commit/a7fdf9600fe1adf3f8f4e90a2172690185788936))
* migrate end-to-end tests to the Rust CLI; fix key derivation, stats, wrong-password ([daf1d7d](https://github.com/corrodedHash/diaria-rs/commit/daf1d7d1c4375b19495dca9d25cac315c5c52996))
* password collection ([44845cc](https://github.com/corrodedHash/diaria-rs/commit/44845cc453e82327e1a9a8f083a664c6b54c91d4))
* print command errors via Display, not Debug ([a97c26b](https://github.com/corrodedHash/diaria-rs/commit/a97c26ba9fc17088138243b1e36bfcdf81f7a851))
* status command ([de32638](https://github.com/corrodedHash/diaria-rs/commit/de326389feb20c10e1508264ddcd6ff155fc0288))
* suppress 'no entry' color for future days in stats heatmap ([#13](https://github.com/corrodedHash/diaria-rs/issues/13)) ([5b84920](https://github.com/corrodedHash/diaria-rs/commit/5b8492021b1d819fa79b7fe701b33352b561a911))
* version the vault setup, not just entries ([99638b1](https://github.com/corrodedHash/diaria-rs/commit/99638b1833337c7259613e56104e653b44beb5a5))
* working init, add and read ([654a779](https://github.com/corrodedHash/diaria-rs/commit/654a779d8a9768a888f5e194c534ef3a9ec16d68))
* zeroize keys and enable aur publishing ([9862286](https://github.com/corrodedHash/diaria-rs/commit/9862286e450aea4cfdf327f3b65b132f9c862ea6))


### Bug Fixes

* Docker image build — correct mise install env var and add trust step ([4c8343f](https://github.com/corrodedHash/diaria-rs/commit/4c8343feaa801ae3f9d9cb1f9be632309d5d7994))
* ensure load command stores entries with .diaria extension ([2cd7a67](https://github.com/corrodedHash/diaria-rs/commit/2cd7a675d18fc8152f37e3cce0f837408d363735))
* even more release ([#10](https://github.com/corrodedHash/diaria-rs/issues/10)) ([2dbb79a](https://github.com/corrodedHash/diaria-rs/commit/2dbb79a45d95a36e23e59e108bf11e1219f0e4c7))
* idempotent release ([#8](https://github.com/corrodedHash/diaria-rs/issues/8)) ([ef30f43](https://github.com/corrodedHash/diaria-rs/commit/ef30f4311e35f491cacb93b8337a2e0a357e76e1))
* made add subcommand more stable ([d954865](https://github.com/corrodedHash/diaria-rs/commit/d95486577efd639a9290a5e0c6814a8a5b526712))
* only list entries in read command ([c684b2e](https://github.com/corrodedHash/diaria-rs/commit/c684b2e12b1bfea9a84a69552010529d43e33f7e))
* opencode entrypoint — binary not on PATH and state dir missing ([f3dc22f](https://github.com/corrodedHash/diaria-rs/commit/f3dc22f89b5a3e2f5a7c53d502eae21ee5e9e3ea))
* push release tag only after PR merges to main ([#4](https://github.com/corrodedHash/diaria-rs/issues/4)) ([a3feade](https://github.com/corrodedHash/diaria-rs/commit/a3feadec7a534f547fdf14f7cbfca630aa0444dd))
* release script uses correct PR for tagging ([#6](https://github.com/corrodedHash/diaria-rs/issues/6)) ([8555005](https://github.com/corrodedHash/diaria-rs/commit/8555005270eb8e3f8dad6679edc6073009df9faa))
* **release:** tag without committing when version is unchanged ([c4e7ae9](https://github.com/corrodedHash/diaria-rs/commit/c4e7ae93f359ecfa03c12f0bb5b1bd7dd82ea46f))
* resolve relative XDG_DATA_HOME to absolute path ([c71c84b](https://github.com/corrodedHash/diaria-rs/commit/c71c84bb0e97a7f0148d9db1e55da3763f6f8f7a))
* satisfy strict clippy lints in test modules ([5288384](https://github.com/corrodedHash/diaria-rs/commit/528838475b11136895faa5bd03b3997461e638d3))
* set repo-url explicitly in gh release upload ([#16](https://github.com/corrodedHash/diaria-rs/issues/16)) ([4f39a55](https://github.com/corrodedHash/diaria-rs/commit/4f39a5556849c9e31f0676b87082deaaac9e446b))
* **stats:** group heatmap by year and split stats module ([8ca3808](https://github.com/corrodedHash/diaria-rs/commit/8ca380857112cc4094add02ddaa02f72f28a68c8))
* suppress component prefix in git tags ([#19](https://github.com/corrodedHash/diaria-rs/issues/19)) ([3f19eb9](https://github.com/corrodedHash/diaria-rs/commit/3f19eb92a149d1668f171b593665cf16f299ee74))
* use v-prefixed tags instead of package-prefixed tags ([#18](https://github.com/corrodedHash/diaria-rs/issues/18)) ([df57a78](https://github.com/corrodedHash/diaria-rs/commit/df57a78790edd3a2976dbb98a7c9935907aead21))

## [1.1.1](https://github.com/corrodedHash/diaria-rs/compare/diaria-v1.1.0...diaria-v1.1.1) (2026-07-19)


### Bug Fixes

* set repo-url explicitly in gh release upload ([#16](https://github.com/corrodedHash/diaria-rs/issues/16)) ([4f39a55](https://github.com/corrodedHash/diaria-rs/commit/4f39a5556849c9e31f0676b87082deaaac9e446b))

## [1.1.0](https://github.com/corrodedHash/diaria-rs/compare/diaria-v1.0.3...diaria-v1.1.0) (2026-07-19)


### Features

* add free-tier CI release pipeline ([9539e06](https://github.com/corrodedHash/diaria-rs/commit/9539e061e00eaf73f6bc9e9ef8ec93923c988a25))
* compression ([86d9bec](https://github.com/corrodedHash/diaria-rs/commit/86d9becf225e0ed0427bf3c2db19c339dc635c14))
* implement custom pacman repo push ([d12cdfb](https://github.com/corrodedHash/diaria-rs/commit/d12cdfbf8b328e60fceabdd53d46e793b4ca0b9d))
* implement docker container sandboxed agent ([1b21765](https://github.com/corrodedHash/diaria-rs/commit/1b2176523cda8f3c0b082174440617f63bfcc8fc))
* initial encryption / encoding ([31c1de9](https://github.com/corrodedHash/diaria-rs/commit/31c1de97579120f2754f076c4a8f72742d0cbe02))
* initial implementation of all subcommands ([893011c](https://github.com/corrodedHash/diaria-rs/commit/893011ca235330b92a88e1bf7b3e2217f65af27b))
* inject UserOutput into DiariaFsRepository for warnings ([4a274ca](https://github.com/corrodedHash/diaria-rs/commit/4a274ca6c91250c2681a712e80400f0b0867e5ca))
* made private key encryption less weird ([a7fdf96](https://github.com/corrodedHash/diaria-rs/commit/a7fdf9600fe1adf3f8f4e90a2172690185788936))
* migrate end-to-end tests to the Rust CLI; fix key derivation, stats, wrong-password ([daf1d7d](https://github.com/corrodedHash/diaria-rs/commit/daf1d7d1c4375b19495dca9d25cac315c5c52996))
* password collection ([44845cc](https://github.com/corrodedHash/diaria-rs/commit/44845cc453e82327e1a9a8f083a664c6b54c91d4))
* print command errors via Display, not Debug ([a97c26b](https://github.com/corrodedHash/diaria-rs/commit/a97c26ba9fc17088138243b1e36bfcdf81f7a851))
* status command ([de32638](https://github.com/corrodedHash/diaria-rs/commit/de326389feb20c10e1508264ddcd6ff155fc0288))
* suppress 'no entry' color for future days in stats heatmap ([#13](https://github.com/corrodedHash/diaria-rs/issues/13)) ([5b84920](https://github.com/corrodedHash/diaria-rs/commit/5b8492021b1d819fa79b7fe701b33352b561a911))
* version the vault setup, not just entries ([99638b1](https://github.com/corrodedHash/diaria-rs/commit/99638b1833337c7259613e56104e653b44beb5a5))
* working init, add and read ([654a779](https://github.com/corrodedHash/diaria-rs/commit/654a779d8a9768a888f5e194c534ef3a9ec16d68))
* zeroize keys and enable aur publishing ([9862286](https://github.com/corrodedHash/diaria-rs/commit/9862286e450aea4cfdf327f3b65b132f9c862ea6))


### Bug Fixes

* Docker image build — correct mise install env var and add trust step ([4c8343f](https://github.com/corrodedHash/diaria-rs/commit/4c8343feaa801ae3f9d9cb1f9be632309d5d7994))
* ensure load command stores entries with .diaria extension ([2cd7a67](https://github.com/corrodedHash/diaria-rs/commit/2cd7a675d18fc8152f37e3cce0f837408d363735))
* even more release ([#10](https://github.com/corrodedHash/diaria-rs/issues/10)) ([2dbb79a](https://github.com/corrodedHash/diaria-rs/commit/2dbb79a45d95a36e23e59e108bf11e1219f0e4c7))
* idempotent release ([#8](https://github.com/corrodedHash/diaria-rs/issues/8)) ([ef30f43](https://github.com/corrodedHash/diaria-rs/commit/ef30f4311e35f491cacb93b8337a2e0a357e76e1))
* made add subcommand more stable ([d954865](https://github.com/corrodedHash/diaria-rs/commit/d95486577efd639a9290a5e0c6814a8a5b526712))
* only list entries in read command ([c684b2e](https://github.com/corrodedHash/diaria-rs/commit/c684b2e12b1bfea9a84a69552010529d43e33f7e))
* opencode entrypoint — binary not on PATH and state dir missing ([f3dc22f](https://github.com/corrodedHash/diaria-rs/commit/f3dc22f89b5a3e2f5a7c53d502eae21ee5e9e3ea))
* push release tag only after PR merges to main ([#4](https://github.com/corrodedHash/diaria-rs/issues/4)) ([a3feade](https://github.com/corrodedHash/diaria-rs/commit/a3feadec7a534f547fdf14f7cbfca630aa0444dd))
* release script uses correct PR for tagging ([#6](https://github.com/corrodedHash/diaria-rs/issues/6)) ([8555005](https://github.com/corrodedHash/diaria-rs/commit/8555005270eb8e3f8dad6679edc6073009df9faa))
* **release:** tag without committing when version is unchanged ([c4e7ae9](https://github.com/corrodedHash/diaria-rs/commit/c4e7ae93f359ecfa03c12f0bb5b1bd7dd82ea46f))
* resolve relative XDG_DATA_HOME to absolute path ([c71c84b](https://github.com/corrodedHash/diaria-rs/commit/c71c84bb0e97a7f0148d9db1e55da3763f6f8f7a))
* satisfy strict clippy lints in test modules ([5288384](https://github.com/corrodedHash/diaria-rs/commit/528838475b11136895faa5bd03b3997461e638d3))
* **stats:** group heatmap by year and split stats module ([8ca3808](https://github.com/corrodedHash/diaria-rs/commit/8ca380857112cc4094add02ddaa02f72f28a68c8))
