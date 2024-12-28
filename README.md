# async-rt

## Overview

async-rt is a small library designed to utilize async executors (i.e Tokio) through a common API while extending functionality
for abortable task, tasks that receive messages and being able to switch between two async runtimes at compile time if specific
conditions are met (i.e tokio if the feature is enabled and its a non-wasm32 arch, while wasm32-bindgen-futures is used if the arch is wasm32).

## MSRV

The minimum supported rust version is 1.83, which can be changed in the future. There is no guarantee that this library will work on older versions of rust.

## License

This crate is licensed under either Apache 2.0 or MIT. 