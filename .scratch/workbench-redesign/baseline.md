# Baseline verification

Source: 9f678b920cc88e13760ebb0cb2be43bf8f042cc4.

Command: `CARGO_TARGET_DIR=/tmp/sofdevtool-redesign-target make verify` in the managed worktree.

Result: passed. Formatting, Clippy, 113 app tests, 190 core tests, 23 JSON contracts, 9 retained vectors, 5 packaging tests and Debug build passed. Existing transitive `block v0.1.6` future-incompatibility warning remains.

This is automated baseline evidence, not native UI acceptance. A copy-on-write clone of the existing target cache is used exclusively by this worktree.
