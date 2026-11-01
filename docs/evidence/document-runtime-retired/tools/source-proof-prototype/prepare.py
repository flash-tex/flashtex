"""Prepare isolated crate; production source is copied, never changed."""
import hashlib,json,os,shutil,subprocess
from pathlib import Path
repo=Path(__file__).resolve().parents[4]
crate=repo/'crates/document-runtime'
out=Path('/home/natkarri/flashtex-source-proof-prototype')
(out/'crates').mkdir(parents=True,exist_ok=True)
scratch=out/'crates/document-runtime';scratch.mkdir(exist_ok=True)
for name in ['Cargo.toml','Cargo.lock']:shutil.copy2(crate/name,scratch/name)
shutil.copytree(crate/'src',scratch/'src',dirs_exist_ok=True)
for name in ['fixtures','benchmarks']:
 if not (scratch/name).exists():(scratch/name).symlink_to(crate/name)
for dep in (repo/'crates').iterdir():
 if dep.name!='document-runtime' and not (out/'crates'/dep.name).exists():(out/'crates'/dep.name).symlink_to(dep)
source=(crate/'src/lib.rs').read_text()
start=source.index('fn validate_reply_value(');end=source.index('\npub fn validate_layout_capabilities',start)
original=source[start:end]
replacement=original.replace('fn validate_reply_value(', 'fn validate_reply_proof(').replace('r: &Request','r: &ProofRequest').replace('BTreeMap<&str, &str>','BTreeMap<&str, &Proof>').replace('document.text.as_str()','&document.text')
prototype=(Path(__file__).with_name('prototype.rs')).read_text()
(scratch/'src/source_proof_prototype.rs').write_text(prototype+'\n'+replacement)
(scratch/'src/lib.rs').write_text(source+'\n#[cfg(test)] mod source_proof_prototype;\n')
import gzip
captured=crate/'benchmarks/source-proof-prototype/cases.json.gz'
(out/'cases.json').write_bytes(gzip.decompress(captured.read_bytes()))
rows=json.loads((out/'cases.json').read_text())
evidence={'runtime_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'original_validator_sha256':hashlib.sha256(original.encode()).hexdigest(),'generated_validator_sha256':hashlib.sha256(replacement.encode()).hexdigest(),'cases':len(rows),'cases_sha256':hashlib.sha256((out/'cases.json').read_bytes()).hexdigest(),'scope':'scratch copy only, original validator mechanically changes source-map type/access only'}
(out/'prepare.json').write_text(json.dumps(evidence,indent=2)+'\n')
print(json.dumps(evidence))
