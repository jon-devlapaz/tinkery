"""Bounded official-viewer startup diagnostic; private TEST session, no browser."""
import json, os, subprocess, tempfile
from pathlib import Path
skill = Path(os.environ['SEED_ME_TEST_SKILL']).resolve()
helper = skill.parent / 'scripts/session.py'
viewer = skill.parent / 'scripts/viewer.py'
with tempfile.TemporaryDirectory(prefix='tinkery-TEST-viewer-probe-') as root:
    result = subprocess.run(['python3', str(helper), 'init', '--root', root], capture_output=True, text=True, check=True)
    session = json.loads(result.stdout)['session']
    wrapper = 'import faulthandler,runpy,sys;faulthandler.dump_traceback_later(5,repeat=True);sys.path.insert(0,sys.argv[1]);sys.argv=sys.argv[2:];runpy.run_path(sys.argv[0],run_name="__main__")'
    child = subprocess.Popen(['python3', '-c', wrapper, str(viewer.parent), str(viewer), session], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        stdout, stderr = child.communicate(timeout=8)
    except subprocess.TimeoutExpired:
        child.kill()
        stdout, stderr = child.communicate()
    print(json.dumps(dict(diagnostic='official viewer, killed only owned TEST process after 8s',stdout=stdout,stderr=stderr), ensure_ascii=False))
