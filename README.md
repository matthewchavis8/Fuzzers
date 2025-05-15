# Example Fuzzers

The goal is to demonstrate how to build effective fuzzers for different environments from simple in-memory fuzzing setups to more complex use cases like bare-metal targets or embedded systems.

## Structure

Each folder in this repository is a self-contained Rust project (`cargo new`) representing a specific fuzzing scenario:

- `baby_fuzzer` – A minimal fuzzer showcasing coverage-guided fuzzing using `InProcessExecutor`, `StdFuzzer`, and basic mutation stages.
- `baby_fuzzer_with_custom_executor` – A minimal fuzzer showcasing coverage-guided fuzzing using a custom executor and also with a bloom input filter and multiple stages such as calibration stage and AflStats stage.
- 'fuzzing_c_code_inprocess_executor' A fuzzer with basic coverage guided fuzzing but this time instrumented and calling actual C code
- 'fuzzing_c_code_with_fork_executor' Same as above but will fork instead of running it in the same process
- 'fuzzing_baremetal' - A QEMU-based fuzzer that feeds random inputs into a ARM based bare-metal firmware, tracks every code path for coverage, and flags crashes or hangs automatically
supports sync_exit, low_level, or breakpoint. This one is pretty cool!
- *(More coming soon...)*

## What is LibAFl?
LibAFl is a fuzzing library written in Rust. LibAfl is kind of like a jack of all trades of fuzzers meaning it is very customizable. Libafl is how I put it like building with legos haha it is pretty cool!:)

## 📦 Requirements

- Rust (2021 or 2024 edition)
- `cargo install cargo-binutils` and `rustup component add llvm-tools-preview` for coverage instrumentation (optional)
- [LibAFL](https://crates.io/crates/libafl)
