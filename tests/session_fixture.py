"""TEST-only transport to Tinkery's actual Rust ledger; no verifier or formatter copies."""
import json
import os
from pathlib import Path
import test_launch as subprocess

REPO = Path(__file__).resolve().parents[1]
BINARY = REPO / 'target/debug/examples/seed_fixture'
# The parallel runner owns preparation; standalone journeys still prepare themselves.
if os.environ.get('TINKERY_TEST_SEED_FIXTURE') != str(BINARY):
    subprocess.run(['cargo', '+stable', 'build', '--locked', '--example', 'seed_fixture'],
                   cwd=REPO, check=True, stdout=subprocess.DEVNULL, env=subprocess.environment())
elif not BINARY.is_file():
    raise FileNotFoundError(f'Prepared TEST seed fixture is missing: {BINARY}')


def call(op, path='', value=None, **fields):
    request = dict(op=op, path=str(path), value=value, **fields)
    result = subprocess.run([str(BINARY)], input=json.dumps(request), text=True,
                            capture_output=True, check=True, env=subprocess.environment())
    return json.loads(result.stdout)


def create(root, operator='human'):
    return Path(call('init', Path(root).resolve(), operator=operator)['session'])


def load(path):
    return call('read', path)


def editable(value):
    return call('editable', value=value)


def review_flags(value):
    return call('flags', value=value)


def decisions_block(value):
    return call('decisions', value=value)


def checks_block(value):
    return call('render', value=value)


def read_checks(path):
    return call('checks', path)


def set_checks(path, value):
    return call('set-checks', path, value)


def status(path):
    return call('status', path)


def publish(path, state, expected, reason):
    return call('publish', path, state, expected=expected, reason=reason)


def confirm_seed(path, revision, source):
    return call('confirm', path, revision=revision, source=source)


def end(path, status, reason):
    return call('end', path, status=status, reason=reason)
