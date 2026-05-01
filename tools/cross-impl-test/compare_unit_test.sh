#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/compare.sh"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

cat > "$TMP_DIR/run1.txt" <<'EOF'
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 149 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 63.51s
EOF

cat > "$TMP_DIR/run2.txt" <<'EOF'
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 149 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 62.62s
EOF

normalize_rust_test_results "$TMP_DIR/run1.txt" "$TMP_DIR/run1.normalized"
normalize_rust_test_results "$TMP_DIR/run2.txt" "$TMP_DIR/run2.normalized"
diff -u "$TMP_DIR/run1.normalized" "$TMP_DIR/run2.normalized"

cat > "$TMP_DIR/run3.txt" <<'EOF'
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 149 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 62.62s
EOF

normalize_rust_test_results "$TMP_DIR/run3.txt" "$TMP_DIR/run3.normalized"
if diff -q "$TMP_DIR/run1.normalized" "$TMP_DIR/run3.normalized" > /dev/null; then
    echo "normalization masked a substantive test-count difference" >&2
    exit 1
fi

RESULTS_DIR="$TMP_DIR/results"
VECTORS_DIR="$TMP_DIR/vectors"
DETERMINISM_STATUS="Failed (normalized Rust test summaries differ)"
mkdir -p "$RESULTS_DIR/rust" "$VECTORS_DIR"
printf '6\n' > "$RESULTS_DIR/rust/pass_count"
printf '6\n' > "$RESULTS_DIR/rust/total_count"

generate_report
grep -F "Determinism: Failed (normalized Rust test summaries differ)" \
    "$RESULTS_DIR/report.txt" > /dev/null
