"""Print the ledger's CPython 3.13 character tables; refuse other Unicode versions."""
import sys
import unicodedata


def main():
    if sys.version_info[:2] != (3, 13) or unicodedata.unidata_version != "15.1.0":
        raise SystemExit("use CPython 3.13 with Unicode 15.1.0 to regenerate unicode.rs")
    print("//! CPython 3.13 word and repr character boundaries (Unicode 15.1.0).")
    print("//! Regenerate with Python 3.13: tests/ledger/generate_unicode.py.")
    for name, predicate in (("ALNUM", str.isalnum), ("NONPRINTABLE", lambda c: not c.isprintable())):
        print(f"\npub(super) const {name}: &[(u32, u32)] = &[")
        start = None
        for code in range(0x110001):
            matches = code < 0x110000 and predicate(chr(code))
            if matches and start is None:
                start = code
            elif not matches and start is not None:
                print(f"    (0x{start:x}, 0x{code - 1:x}),")
                start = None
        print("];")


if __name__ == "__main__":
    main()
