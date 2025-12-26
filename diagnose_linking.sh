#!/bin/bash
# Diagnostic script for risc0 linking issues on macOS ARM

echo "=== risc0 Linking Diagnostics ==="
echo ""

echo "1. System Information:"
echo "  Architecture: $(uname -m)"
echo "  OS: $(uname -s) $(uname -r)"
echo ""

echo "2. Rust Toolchain:"
rustc --version
rustup show active-toolchain
echo ""

echo "3. risc0 Installation:"
rzup show
echo ""

echo "4. Check for iconv conflicts:"
if command -v port &> /dev/null; then
    echo "  MacPorts found:"
    port installed | grep iconv || echo "  No iconv from MacPorts"
else
    echo "  MacPorts not installed"
fi
if command -v brew &> /dev/null; then
    echo "  Homebrew found:"
    brew list | grep iconv || echo "  No iconv from Homebrew"
else
    echo "  Homebrew not installed"
fi
echo ""

echo "5. Circuit library files in build:"
find target/debug/build -name "*risc0*circuit*" -type f 2>/dev/null | head -20
echo ""

echo "6. Checking for C++ kernel object files:"
find target/debug/build -name "*.o" -path "*risc0-circuit*" 2>/dev/null | head -20
echo ""

echo "7. Environment variables:"
env | grep -i risc0
echo ""

echo "8. Attempting minimal build test:"
cargo clean -p ror
echo "  Building host package only..."
cargo build --bin ror 2>&1 | tail -20
echo ""

echo "9. Checking if circuit symbols exist in libraries:"
for lib in target/debug/build/risc0-circuit-*/out/*.a 2>/dev/null; do
    if [ -f "$lib" ]; then
        echo "  Checking $lib:"
        nm "$lib" | grep -E "(cpu_poly|cpu_witgen|cpu_accum|cpu_eval)" | head -5
    fi
done
echo ""

echo "10. DYLD library paths:"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-not set}"
echo "  DYLD_FALLBACK_LIBRARY_PATH: ${DYLD_FALLBACK_LIBRARY_PATH:-not set}"
echo ""

echo "=== Diagnostics Complete ==="
echo ""
echo "To run proof generation after reviewing diagnostics:"
echo "  ./generate_proof.sh"
