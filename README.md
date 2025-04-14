# Example Fuzzers

The goal is to demonstrate how to build effective fuzzers for different environments from simple in-memory fuzzing setups to more complex use cases like bare-metal targets or embedded systems.

## 📁 Structure

Each folder in this repository is a self-contained Rust project (`cargo new`) representing a specific fuzzing scenario:

- `baby_fuzzer/` – A minimal fuzzer showcasing coverage-guided fuzzing using `InProcessExecutor`, `StdFuzzer`, and basic mutation stages.
- *(More coming soon...)*

## What is LibAFl?
LibAFl is a fuzzing library written in Rust. LibAfl is kind of like a jack of all trades of fuzzers meaning it is very customizable. Libafl is how I put it like building with legos haha it is pretty cool!:)

## 📦 Requirements

- Rust (2021 or 2024 edition)
- `cargo install cargo-binutils` and `rustup component add llvm-tools-preview` for coverage instrumentation (optional)
- [LibAFL](https://crates.io/crates/libafl)
