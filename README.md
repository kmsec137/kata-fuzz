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

## Agent Source Modifications

To allow the fuzzing harnesses to construct and interact with the agent's internal state directly, you must make a few slight modifications to the Kata agent source code before compiling. 

Specifically, you need to change certain struct properties and initialization functions from private (`pub(crate)` or omitted) to public (`pub`). 
* Ensure the fields inside `Sandbox` and `AgentService` that the harnesses access are marked `pub`.
* Expose any required internal parsing or storage handler methods so the fuzzer can call them directly.

---

## Build & Setup Instructions

### Prerequisites
* Rust Nightly toolchain
* Cloned [Kata Containers](https://github.com/kata-containers/kata-containers) repository[cite: 6]

### Setup Steps

**1. Navigate to the agent directory**
Move into the agent directory of your cloned repository:[cite: 6]
```bash
cd src/agent
