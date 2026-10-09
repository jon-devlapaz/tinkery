from pathlib import Path
import argparse,json,shutil,subprocess,tempfile
parser=argparse.ArgumentParser();parser.add_argument('--repo',type=Path,required=True);args=parser.parse_args();repo=args.repo.resolve()
root=Path(tempfile.mkdtemp(prefix='tinkery-TEST-subtraction-census-probes-'));(root/'scripts').mkdir();(root/'tests').mkdir()
shutil.copy2(repo/'scripts/rejection_census.py',root/'scripts/rejection_census.py');shutil.copy2(repo/'tests/test_rejection_census.py',root/'tests/test_rejection_census.py');shutil.copytree(repo/'docs',root/'docs')
p=root/'scripts/rejection_census.py';original=p.read_text()
def run(name):
 r=subprocess.run(['python3','tests/test_rejection_census.py'],cwd=root,capture_output=True,text=True,timeout=30);(root/(name+'.log')).write_text(r.stdout+r.stderr);return r
assert run('baseline').returncode==0
cases=[('extra-fields-as-parse','return "S", "exact-object-keys"','return "P", "exact-object-keys"'),('model-veto-as-proven-fact','model allegation; not a proven fact violation','proven deterministic fact violation'),('duplicate-state-inflation','            peers[0]["aliases"].append(notice["artifact"])','            events.append(notice)')]
for name,old,new in cases:
 assert original.count(old)==1;p.write_text(original.replace(old,new));r=run(name);assert r.returncode and 'FAIL' in r.stderr and 'ERROR' not in r.stderr;print(name,'detected')
(root/'result.json').write_text(json.dumps({'baseline':True,'proven':[c[0] for c in cases],'scope':'isolated offline census regressions, no providers/helpers'},indent=2)+'\n');print(root)
