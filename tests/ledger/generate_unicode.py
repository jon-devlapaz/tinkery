"""Print the ledger's CPython 3.13 word-character table; refuse other Unicode versions."""
import sys
import unicodedata


def main():
    if sys.version_info[:2] != (3, 13) or unicodedata.unidata_version != "15.1.0":
        raise SystemExit("use CPython 3.13 with Unicode 15.1.0 to regenerate unicode.rs")
    print("//! CPython 3.13 str.isalnum (Unicode 15.1.0); combining marks are not word characters.")
    print("//! Regenerate with Python 3.13: tests/ledger/generate_unicode.py.")
    print("\npub(super) const ALNUM: &[(u32, u32)] = &[")
    start = None
    for code in range(0x110001):
        alnum = code < 0x110000 and chr(code).isalnum()
        if alnum and start is None:
            start = code
        elif not alnum and start is not None:
            print(f"    (0x{start:x}, 0x{code - 1:x}),")
            start = None
    print("];")


if __name__ == "__main__":
    main()
