# Kata-Fuzz

**Kata-Fuzz** is a collection of simple coverage-guided fuzzing harnesses targeting the ttRPC interface between the host container runtime (`containerd-shim-kata-v2`) and the guest agent in Kata Containers.

---

## Construction

Kata-Fuzz integrates **libFuzzer** (`cargo-fuzz`) directly into the Kata Containers agent source tree (`src/agent`). By compiling against the agent's internal modules (`rpc.rs`, `sandbox.rs`, `storage.rs`), it achieves high-throughput, in-process fuzzing for critical RPC endpoints.

### Key Architectural Features
* **Persistent Runtime & Sandbox State:** Leverages `OnceLock` to initialize the `tokio` multi-threaded runtime, `Sandbox`, and `AgentService` state exactly once.
* **Resource Exhaustion Guard:** Hoists netlink (`rtnl`) socket creation out of the fuzzing loop to prevent file descriptor (`ulimit`) leaks across millions of iterations.
* **In-Process Parsing Execution:** Feeds mutated ttRPC protobuf payloads directly into internal deserialization and state handling routines.

---

## Build & Setup Instructions

### Prerequisites
* Rust Nightly toolchain
* `cargo-fuzz` (`cargo install cargo-fuzz`)
* Cloned [Kata Containers](https://github.com/kata-containers/kata-containers) repository

### Setup Steps

1. **Navigate to the agent directory:**
   ```bash
   cd src/agent
	```

2. Initialize the fuzzing environment
If you haven't already set up cargo-fuzz in this workspace, install the tool and initialize it:

Bash
cargo install cargo-fuzz
cargo fuzz init
3. Deploy the fuzzing harnesses
Copy your custom fuzzing files into the newly created fuzz_targets directory:

Plaintext
src/agent/fuzz/fuzz_targets/create_container_fuzzer.rs
src/agent/fuzz/fuzz_targets/update_container_fuzzer.rs
src/agent/fuzz/fuzz_targets/exec_process_fuzzer.rs
src/agent/fuzz/fuzz_targets/copy_file_fuzzer.rs
src/agent/fuzz/fuzz_targets/add_storage_fuzzer.rs
4. Register the binary targets
Open src/agent/fuzz/Cargo.toml and append a [[bin]] block for each target so cargo fuzz recognizes them:

Ini, TOML
[[bi
name = "create_container_fuzzer"
path = "fuzz_targets/create_container_fuzzer.rs"
test = false
doc = false

[[bin]]
name = "update_container_fuzzer"
path = "fuzz_targets/update_container_fuzzer.rs"
test = false
doc = false

[[bin]]
name = "exec_process_fuzzer"
path = "fuzz_targets/exec_process_fuzzer.rs"
test = false
doc = false

[[bin]]
name = "copy_file_fuzzer"
path = "fuzz_targets/copy_file_fuzzer.rs"
test = false
doc = false

[[bin]]
name = "add_storage_fuzzer"
path = "fuzz_targets/add_storage_fuzzer.rs"
test = false
doc = false
Running the Fuzzers
Execute any of the configured targets using the Rust nightly toolchain:

Bash
cargo +nightly fuzz run create_container_fuzzer
Advanced Execution
Bypass Host Libc/Netlink Constraints
If your host throws rustix or netlink binding errors during execution, pass the libc configuration flag:

Bash
RUSTFLAGS="--cfg rustix_use_libc" cargo +nightly fuzz run update_container_fuzzer
Multi-Core Fuzzing
To utilize multiple CPU cores for significantly faster throughput, use the jobs and workers flags:

Bash
cargo +nightly fuzz run exec_process_fuzzer -- -jobs=4 -workers=4
Crash Minimization
If the fuzzer discovers a panic, use tmin to reduce the payload to the smallest reproducible byte sequence:

Bash
cargo +nightly fuzz tmin create_container_fuzzer artifacts/create_container_fuzzer/crash-<hash>
