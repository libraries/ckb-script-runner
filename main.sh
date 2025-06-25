set -ex

cargo run --example ckbez_default
cargo run --example ckbez_syscalls_patch
cargo run --example mock_default
cargo run --example mock_syscalls_patch
