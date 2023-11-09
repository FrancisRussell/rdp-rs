### Unreleased
#### Features
* Make error types implement `std::error::Error`.
#### Code changes
* Update code to Rust 2021 edition.
* Bump a number of dependencies to current versions.
* Apply `clippy` fixes.
* Simplify `mstsc-rs` command-line parsing using Clap's derive functionality.
* Eliminate complex `yasna` wrapping code and replace with `rasn`.
* Significantly reduce number of `unwrap`s/`expect`s in CredSSP negotiation.
* Clean up and reduce number of unwraps in run-length encoding code.
#### Bug fixes
* Fix potential truncated read in `core::per::read_padding`.
* Fix potential truncated write in `<Vec<u8> as Message>::write`.
* Fix potential truncated write in `model::link::Stream::write` (now renamed).
* Fix multiple potential truncated/oversized reads in `nla::cssp::cssp_connect`.
* Fix oversized decode buffer in `BitmapEvent::decompress`.

### 0.1.1 (2020-04-11)
#### Features
* Remove dependency of `rust-crypto`.
* Fix parameter name.
* Fix overflow in packet computation.

### 0.1.0 (2020-04-11)
#### Features
* Initial release.
