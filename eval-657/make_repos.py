#!/usr/bin/env python3
"""#657 eval: five small TEST repos, each with one failing test for a small bug.

In four, the bare test command fails as cloned (setup lives in tox, CI, a Makefile or npm
workspaces); the cargo workspace is a control where the bare command works. Deterministic, so
the Mac (where Shape reads them) and the cloud (where commands are scored) build identical copies.

    python3 make_repos.py OUT
"""
import json
import subprocess
import sys
from pathlib import Path

REPOS = {}


def repo(name, goal, probe):
    """probe: (relative test path template, body) the scorer writes when a proposed new test file is absent."""
    def add(files):
        REPOS[name] = dict(goal=goal, probe=probe, files=files)
    return add


repo('tox-src', goal=(
    'minitoml parses an empty inline table.\nWhy: config files with x = {} crash the loader.\n'
    'Done when: tests/test_parse.py test_empty_inline_table passes.\n'
    'Must: Keep the change to the parser.\nMust not: Change the public API.'),
    probe='import unittest\nimport minitoml\n\n\nclass T(unittest.TestCase):\n    def test_probe(self):\n        self.assertTrue(minitoml.loads)\n')({
    'pyproject.toml': '[build-system]\nrequires = ["setuptools>=61"]\nbuild-backend = "setuptools.build_meta"\n\n'
                      '[project]\nname = "minitoml"\nversion = "0.1.0"\n\n[tool.setuptools.packages.find]\nwhere = ["src"]\n',
    'tox.ini': '[tox]\nenvlist = py\n\n[testenv]\ncommands = python -m unittest {posargs}\n',
    '.github/workflows/tests.yml': 'name: tests\non: [push, pull_request]\njobs:\n  test:\n    runs-on: ubuntu-latest\n'
        '    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/setup-python@v5\n'
        '      - run: pip install tox\n      - run: tox -e py\n',
    'src/minitoml/__init__.py': 'from ._parser import loads\n\n__all__ = ["loads"]\n',
    'src/minitoml/_parser.py': (
        '"""A tiny TOML subset: key = value lines, strings, integers and inline tables."""\n\n\n'
        'def _value(text):\n    text = text.strip()\n    if text.startswith(\'"\'):\n        return text[1:-1]\n'
        '    if text.startswith("{"):\n        inner = text[1:-1]\n'
        '        return dict(_pair(part) for part in inner.split(","))\n    return int(text)\n\n\n'
        'def _pair(text):\n    key, value = text.split("=", 1)\n    return key.strip(), _value(value)\n\n\n'
        'def loads(text):\n    return dict(_pair(line) for line in text.splitlines() if line.strip())\n'),
    'tests/__init__.py': '',
    'tests/test_parse.py': (
        'import unittest\n\nimport minitoml\n\n\nclass ParseTests(unittest.TestCase):\n'
        '    def test_integer(self):\n        self.assertEqual(minitoml.loads("a = 1"), {"a": 1})\n\n'
        '    def test_inline_table(self):\n        self.assertEqual(minitoml.loads("a = {b = 1}"), {"a": {"b": 1}})\n\n'
        '    def test_empty_inline_table(self):\n        self.assertEqual(minitoml.loads("a = {}"), {"a": {}})\n'),
    'README.md': '# minitoml\n\nA tiny TOML subset parser. Run the tests with `tox`.\n',
})

repo('generated-version', goal=(
    'semverlite compares pre-release versions correctly.\nWhy: 1.0.0-rc.1 sorts after 1.0.0 today.\n'
    'Done when: tests/test_compare.py test_prerelease_sorts_first passes.\n'
    'Must: Follow semver precedence.\nMust not: Add dependencies.'),
    probe='import unittest\nimport semverlite\n\n\nclass T(unittest.TestCase):\n    def test_probe(self):\n        self.assertTrue(semverlite.compare)\n')({
    'Makefile': 'gen:\n\tpython3 scripts/gen_version.py > src/semverlite/_version.py\n\n'
                'test: gen\n\tPYTHONPATH=src python3 -m unittest discover -s tests -t .\n\n.PHONY: gen test\n',
    '.github/workflows/ci.yml': 'name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n'
        '      - uses: actions/checkout@v4\n      - run: make test\n',
    '.gitignore': 'src/semverlite/_version.py\n',
    'scripts/gen_version.py': 'print(\'VERSION = "0.3.0"\')\n',
    'src/semverlite/__init__.py': 'from ._version import VERSION\nfrom .compare import compare\n\n__all__ = ["VERSION", "compare"]\n',
    'src/semverlite/compare.py': (
        '"""Compare two semver strings: -1, 0 or 1."""\n\n\n'
        'def _key(version):\n    core, _, pre = version.partition("-")\n'
        '    return tuple(int(part) for part in core.split(".")), pre\n\n\n'
        'def compare(a, b):\n    left, right = _key(a), _key(b)\n    return (left > right) - (left < right)\n'),
    'tests/__init__.py': '',
    'tests/test_compare.py': (
        'import unittest\n\nfrom semverlite import compare\n\n\nclass CompareTests(unittest.TestCase):\n'
        '    def test_core(self):\n        self.assertEqual(compare("1.2.0", "1.10.0"), -1)\n\n'
        '    def test_prerelease_sorts_first(self):\n        self.assertEqual(compare("1.0.0-rc.1", "1.0.0"), -1)\n'),
    'README.md': '# semverlite\n\nRun `make test`.\n',
})

repo('npm-workspace', goal=(
    'The slug CLI keeps digits in slugs.\nWhy: "Release 2" becomes "release-" today.\n'
    'Done when: packages/cli/test/slug.test.mjs keeps digits passes.\n'
    'Must: Fix it in @demo/core.\nMust not: Add dependencies.'),
    probe="import test from 'node:test';\nimport assert from 'node:assert/strict';\nimport { slugify } from '@demo/core';\n\n"
          "test('probe', () => assert.equal(typeof slugify, 'function'));\n")({
    'package.json': json.dumps({'name': 'slug-tools', 'private': True, 'workspaces': ['packages/*'],
                                'scripts': {'test': 'npm test --workspaces'}}, indent=2) + '\n',
    'packages/core/package.json': json.dumps({'name': '@demo/core', 'version': '0.1.0', 'type': 'module',
                                              'main': 'index.js', 'scripts': {'test': 'node --test'}}, indent=2) + '\n',
    'packages/core/index.js': "export function slugify(text) {\n  return text.toLowerCase().replace(/[^a-z]+/g, '-');\n}\n",
    'packages/cli/package.json': json.dumps({'name': '@demo/cli', 'version': '0.1.0', 'type': 'module',
                                             'dependencies': {'@demo/core': '0.1.0'},
                                             'scripts': {'test': 'node --test'}}, indent=2) + '\n',
    'packages/cli/index.js': "import { slugify } from '@demo/core';\n\nconsole.log(slugify(process.argv.slice(2).join(' ')));\n",
    'packages/cli/test/slug.test.mjs': (
        "import test from 'node:test';\nimport assert from 'node:assert/strict';\nimport { slugify } from '@demo/core';\n\n"
        "test('lowercases words', () => assert.equal(slugify('Hello World'), 'hello-world'));\n"
        "test('keeps digits', () => assert.equal(slugify('Release 2'), 'release-2'));\n"),
    '.github/workflows/ci.yml': 'name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n'
        '      - uses: actions/checkout@v4\n      - uses: actions/setup-node@v4\n      - run: npm ci\n      - run: npm test\n',
    'package-lock.json': json.dumps({'name': 'slug-tools', 'lockfileVersion': 3, 'requires': True, 'packages': {
        '': {'name': 'slug-tools', 'workspaces': ['packages/*']},
        'node_modules/@demo/cli': {'resolved': 'packages/cli', 'link': True},
        'node_modules/@demo/core': {'resolved': 'packages/core', 'link': True},
        'packages/cli': {'name': '@demo/cli', 'version': '0.1.0', 'dependencies': {'@demo/core': '0.1.0'}},
        'packages/core': {'name': '@demo/core', 'version': '0.1.0'}}}, indent=2) + '\n',
    'README.md': '# slug-tools\n\n`npm ci && npm test`.\n',
})

repo('makefile-lib', goal=(
    'wordcount counts hyphenated words once.\nWhy: "well-known" counts as two words today.\n'
    'Done when: tests/test_count.py test_hyphenated_word passes.\n'
    'Must: Keep counting whitespace-separated words.\nMust not: Change the CLI output format.'),
    probe='import unittest\nimport wordcount\n\n\nclass T(unittest.TestCase):\n    def test_probe(self):\n        self.assertTrue(wordcount.count)\n')({
    'Makefile': 'test:\n\tPYTHONPATH=lib python3 -m unittest discover -s tests\n\n.PHONY: test\n',
    'lib/wordcount.py': 'import re\n\n\ndef count(text):\n    return len(re.findall(r"[A-Za-z]+", text))\n',
    'tests/test_count.py': (
        'import unittest\n\nimport wordcount\n\n\nclass CountTests(unittest.TestCase):\n'
        '    def test_spaces(self):\n        self.assertEqual(wordcount.count("a b  c"), 3)\n\n'
        '    def test_hyphenated_word(self):\n        self.assertEqual(wordcount.count("a well-known fact"), 3)\n'),
    'README.md': '# wordcount\n\nRun `make test`.\n',
})

repo('cargo-workspace', goal=(
    'clamp keeps values at the upper bound.\nWhy: clamp(10, 0, 10) returns 9 today.\n'
    'Done when: the clampcore test upper_bound_is_inclusive passes.\n'
    'Must: Fix it in clampcore.\nMust not: Add dependencies.'),
    probe=None)({
    'Cargo.toml': '[workspace]\nmembers = ["crates/clampcore", "crates/clampcli"]\nresolver = "2"\n',
    'crates/clampcore/Cargo.toml': '[package]\nname = "clampcore"\nversion = "0.1.0"\nedition = "2021"\n',
    'crates/clampcore/src/lib.rs': (
        '/// Clamp `value` into `low..=high`.\npub fn clamp(value: i64, low: i64, high: i64) -> i64 {\n'
        '    if value < low {\n        low\n    } else if value >= high {\n        high - 1\n    } else {\n        value\n    }\n}\n\n'
        '#[cfg(test)]\nmod tests {\n    use super::clamp;\n\n    #[test]\n    fn below() {\n        assert_eq!(clamp(-3, 0, 10), 0);\n    }\n\n'
        '    #[test]\n    fn upper_bound_is_inclusive() {\n        assert_eq!(clamp(10, 0, 10), 10);\n    }\n}\n'),
    'crates/clampcli/Cargo.toml': '[package]\nname = "clampcli"\nversion = "0.1.0"\nedition = "2021"\n\n'
                                  '[dependencies]\nclampcore = { path = "../clampcore" }\n',
    'crates/clampcli/src/main.rs': 'fn main() {\n    println!("{}", clampcore::clamp(10, 0, 10));\n}\n',
    '.github/workflows/ci.yml': 'name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n'
        '      - uses: actions/checkout@v4\n      - run: cargo test --workspace\n',
    'README.md': '# clamp\n\n`cargo test --workspace`.\n',
})


def build(out):
    out = Path(out).resolve()
    for name, spec in REPOS.items():
        root = out / name
        root.mkdir(parents=True)
        for path, text in spec['files'].items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text(text)
        env = dict(GIT_AUTHOR_NAME='TEST', GIT_AUTHOR_EMAIL='test@example.invalid',
                   GIT_COMMITTER_NAME='TEST', GIT_COMMITTER_EMAIL='test@example.invalid',
                   GIT_AUTHOR_DATE='2026-11-03T00:00:00Z', GIT_COMMITTER_DATE='2026-11-03T00:00:00Z',
                   PATH='/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin')
        for argv in (['git', 'init', '-q', '-b', 'main'], ['git', 'add', '-A'], ['git', 'commit', '-qm', 'TEST fixture']):
            subprocess.run(argv, cwd=root, env=env, check=True)
        (out / f'{name}.goal.txt').write_text(spec['goal'])


if __name__ == '__main__':
    build(sys.argv[1])
