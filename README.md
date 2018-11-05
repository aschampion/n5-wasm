# N5 [![Build Status](https://travis-ci.org/aschampion/n5-wasm.svg?branch=master)](https://travis-ci.org/aschampion/n5-wasm)

Browser-compatible WASM bindings to the [Rust implementation](https://github.com/aschampion/rust-n5) of the [N5 "Not HDF5" n-dimensional tensor file system storage format](https://github.com/saalfeldlab/n5)

N5 datasets must be available via CORS-compatible HTTP. Compatible with Java N5 Version 2.0.2.

Currently only raw and GZIP compression are supported.

## Build Instructions

This assumes you have [rustup](https://rustup.rs/) installed.

```sh
git clone https://github.com/aschampion/n5-wasm
cd n5-wasm
rustup override set nightly
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
wasm-pack build
```

The built npm package will be in `pkg/`.
