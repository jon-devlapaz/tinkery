#!/usr/bin/env python3
"""Count repeated agent mistakes per ISO week, for #926. Read-only and zero-call.

Usage: python3 -I scripts/mistake-baseline.py START END   (dates as YYYY-MM-DD, END inclusive)

Promises: reads only GitHub (through the logged-in `gh`), ~/.claude/projects and
~/.codex/sessions; writes nothing; prints counts, PR numbers and short session ids, never
message or tool text. Refuses to guess: a source that cannot be read is reported, not zeroed.

Metrics:
  reruns   PRs created in the range whose body, or a PR comment made in the range, says a
           failed CI job was re-run. Plain issues do not count.
  blocked  Foreground minutes in shell calls of at least 45 s: Claude Bash calls without
           run_in_background, outside subagent logs and not rejected at a permission
           prompt (an approved call's prompt time counts); Codex shell calls by their
           reported wall time. Overlapping calls in one session count once, in the week
           each span starts.
  pyc      Sessions whose shell output shows unittest failing the pyc test
           (test_untracked_python_cache_does_not_fail_passing_suite, tink-sdlc#210).
  fifo     PRs merged in the range whose title names a blocking or FIFO read fix
           (#489, #585, #764).

Every week of the range prints, so a zero row is visible. `logs from` names the oldest log
record read: earlier weeks have no log data (Claude Code deletes old transcripts).
"""

import datetime as dt
import json
import pathlib
import re
import subprocess
import sys
from collections import defaultdict

REPOS = ('jon-devlapaz/tinkery', 'jon-devlapaz/tink-sdlc')
LONG = 45
RERUN = re.compile(r're-?r[au]n (the |a )?failed', re.I)
FIFO = re.compile(r'fifo|named pipe|non-?regular|blocking (read|open)|o_nonblock', re.I)
CODEX_SHELL = {'exec', 'shell', 'shell_command', 'exec_command', 'local_shell'}
PYC = re.compile(r'^(FAIL|ERROR): test_untracked_python_cache_does_not_fail_passing_suite \(',
                 re.M)
WALL = re.compile(r'Wall time:? ([\d.]+) seconds|"wall_time_seconds":\s*([\d.]+)')


def week(moment):
    year, number, _ = moment.isocalendar()
    return f'{year}-W{number:02d}'


def when(text):
    return dt.datetime.fromisoformat(text.replace('Z', '+00:00'))


def gh(path):
    out = subprocess.run(['gh', 'api', '--paginate', '--slurp', path],
                         check=True, capture_output=True, text=True).stdout
    return [item for page in json.loads(out) for item in page]


def github(start, end, counts, notes):
    since = start.strftime('%Y-%m-%dT%H:%M:%SZ')
    for repo in REPOS:
        seen = set()
        pulls = gh(f'repos/{repo}/pulls?state=all&sort=created&direction=desc&per_page=100')
        prs = {str(p['number']) for p in pulls}
        for p in pulls:
            at = when(p['created_at'])
            if start <= at < end and RERUN.search(p['body'] or ''):
                seen.add(str(p['number']))
                counts[week(at)]['reruns'] += 1
                notes['reruns'].append(f'{repo.split("/")[1]}#{p["number"]}')
        for c in gh(f'repos/{repo}/issues/comments?since={since}&per_page=100'):
            at = when(c['created_at'])
            number = c['issue_url'].rsplit('/', 1)[1]
            if number not in prs:
                continue
            if start <= at < end and RERUN.search(c['body'] or '') and number not in seen:
                seen.add(number)
                counts[week(at)]['reruns'] += 1
                notes['reruns'].append(f'{repo.split("/")[1]}#{number}')
        for p in pulls:
            if p['merged_at'] and start <= when(p['merged_at']) < end and FIFO.search(p['title']):
                counts[week(when(p['merged_at']))]['fifo'] += 1
                notes['fifo'].append(f'{repo.split("/")[1]}#{p["number"]}')


def text_of(value):
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return '\n'.join(text_of(v) for v in value)
    if isinstance(value, dict):
        return text_of(value.get('text') or value.get('content') or '')
    return ''


def records(path):
    for line in path.open(encoding='utf-8', errors='replace'):
        try:
            yield json.loads(line)
        except ValueError:
            continue


def blocked(spans, counts):
    """Adds the union of (start, seconds) spans, so parallel calls count once."""
    last = None
    for began, seconds in sorted(spans):
        finish = began + dt.timedelta(seconds=seconds)
        if last and began < last:
            began = last
        if finish > began:
            counts[week(began)]['blocked'] += (finish - began).total_seconds() / 60
            last = finish


def claude(path, start, end, counts, oldest):
    calls, pyc, spans = {}, None, []
    side = 'subagents' in path.parts
    for r in records(path):
        content = (r.get('message') or {}).get('content')
        if not isinstance(content, list) or 'timestamp' not in r:
            continue
        at = when(r['timestamp'])
        oldest[0] = min(oldest[0] or at, at)
        for block in content:
            if not isinstance(block, dict):
                continue
            if block.get('type') == 'tool_use':
                skip = (block.get('name') != 'Bash'
                        or bool((block.get('input') or {}).get('run_in_background')))
                calls[block.get('id')] = (at, skip)
            elif block.get('type') == 'tool_result' and block.get('tool_use_id') in calls:
                began, skip = calls.pop(block['tool_use_id'])
                if skip:
                    continue
                seconds = (at - began).total_seconds()
                # A rejected call waited at a permission prompt and ran nothing.
                rejected = (r.get('permissionDecision') or {}).get('decision') == 'reject'
                if not (side or rejected or r.get('isSidechain')) and seconds >= LONG and start <= began < end:
                    spans.append((began, seconds))
                if pyc is None and start <= at < end and PYC.search(text_of(block.get('content'))):
                    pyc = at
    blocked(spans, counts)
    return pyc


def codex(path, start, end, counts, oldest):
    calls, pyc, spans = {}, None, []
    for r in records(path):
        payload = r.get('payload') or {}
        kind = payload.get('type') or ''
        if 'timestamp' not in r or not kind.endswith(('call', 'call_output')):
            continue
        at = when(r['timestamp'])
        oldest[0] = min(oldest[0] or at, at)
        if not kind.endswith('_output'):
            if kind == 'local_shell_call' or payload.get('name') in CODEX_SHELL:
                calls[payload.get('call_id')] = at
        elif payload.get('call_id') in calls:
            began = calls.pop(payload['call_id'])
            output = text_of(payload.get('output'))
            wall = WALL.search(output)
            seconds = float(wall.group(1) or wall.group(2)) if wall else (at - began).total_seconds()
            if seconds >= LONG and start <= began < end:
                spans.append((began, seconds))
            if pyc is None and start <= at < end and PYC.search(output):
                pyc = at
    blocked(spans, counts)
    return pyc


def main(argv):
    if len(argv) != 3:
        sys.exit(__doc__)
    start = dt.datetime.fromisoformat(argv[1]).replace(tzinfo=dt.timezone.utc)
    end = dt.datetime.fromisoformat(argv[2]).replace(tzinfo=dt.timezone.utc) + dt.timedelta(days=1)
    counts = defaultdict(lambda: defaultdict(float))
    notes = defaultdict(list)
    try:
        github(start, end, counts, notes)
    except (OSError, subprocess.CalledProcessError) as error:
        notes['unread'].append(f'GitHub: {error}')
    day = start
    while day < end:
        counts[week(day)]['reruns'] += 0
        day += dt.timedelta(days=1)
    oldest = [None]
    home = pathlib.Path.home()
    for root, read in ((home / '.claude/projects', claude), (home / '.codex/sessions', codex)):
        if not root.is_dir():
            notes['unread'].append(f'{root} is missing')
            continue
        for path in sorted(root.rglob('*.jsonl')):
            if dt.datetime.fromtimestamp(path.stat().st_mtime, dt.timezone.utc) < start:
                continue
            at = read(path, start, end, counts, oldest)
            if at:
                counts[week(at)]['pyc'] += 1
                notes['pyc'].append(path.stem[-8:])
    if oldest[0] is None:
        notes['unread'].append('no log record was read')
    logs = f'{oldest[0]:%Y-%m-%d}' if oldest[0] else 'none'
    print(f'Range {argv[1]} to {argv[2]} (UTC); logs from {logs}')
    print(f'{"week":<10}{"reruns":>8}{"blocked min":>13}{"pyc":>6}{"fifo":>6}')
    total = defaultdict(float)
    for name in sorted(counts):
        row = counts[name]
        for key in ('reruns', 'blocked', 'pyc', 'fifo'):
            total[key] += row[key]
        print(f'{name:<10}{row["reruns"]:>8.0f}{row["blocked"]:>13.0f}{row["pyc"]:>6.0f}{row["fifo"]:>6.0f}')
    print(f'{"total":<10}{total["reruns"]:>8.0f}{total["blocked"]:>13.0f}{total["pyc"]:>6.0f}{total["fifo"]:>6.0f}')
    for key in ('reruns', 'fifo', 'pyc', 'unread'):
        if notes[key]:
            print(f'{key}: {", ".join(notes[key])}')


if __name__ == '__main__':
    main(sys.argv)
