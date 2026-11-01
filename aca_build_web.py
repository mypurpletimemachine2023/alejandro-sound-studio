"""Build the real ACA Sound Studio Rust/WASM app without requiring Trunk.

Uses the same Cargo binary target and wasm-bindgen initialization as the upstream
Trunk pipeline. Set CARGO_HOME, RUSTUP_HOME, CARGO_TARGET_DIR and WASM_BINDGEN_BIN
to the prepared local toolchain. No package substitutes or upstream WASM copying.
"""
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess

ROOT=Path(__file__).resolve().parent
CARGO=Path(os.environ.get('CARGO_HOME',str(Path.home()/'.cargo')))/'bin'/('cargo.exe' if os.name=='nt' else 'cargo')
TARGET=Path(os.environ.get('CARGO_TARGET_DIR',str(ROOT/'target'))).resolve()
BINDGEN=os.environ.get('WASM_BINDGEN_BIN','wasm-bindgen')
version=subprocess.check_output([BINDGEN,'--version'],text=True).strip()
if version!='wasm-bindgen 0.2.129':raise SystemExit('Exactly wasm-bindgen0.2.129 is required; found '+version)
subprocess.run([str(CARGO),'build','--locked','--target','wasm32-unknown-unknown','--release','-p','soundcraft-web'],cwd=ROOT,check=True)
wasm=TARGET/'wasm32-unknown-unknown/release/soundcraft-web.wasm'
if not wasm.is_file():raise SystemExit('Cargo did not produce the actual soundcraft-web binary.')
dist=ROOT/'dist/web';dist.mkdir(parents=True,exist_ok=True)
subprocess.run([BINDGEN,'--target','web','--no-typescript','--out-name','soundcraft_web','--out-dir',str(dist),str(wasm)],cwd=ROOT,check=True)
html=(ROOT/'apps/soundcraft-web/index.html').read_text(encoding='utf-8')
html=re.sub(r'\s*<link data-trunk rel="rust"[^>]+>','',html)
html=re.sub(r'<link data-trunk rel="icon"[^>]+>','<link rel="icon" type="image/svg+xml" href="favicon.svg">',html)
html=html.replace('</body>','''<script type="module">
import init from './soundcraft_web.js';
init({module_or_path: './soundcraft_web_bg.wasm'}).catch(error => {
  const el=document.getElementById('soundcraft_loading');
  if(el){el.textContent='ACA Sound Studio could not start: '+error;el.style.pointerEvents='auto';}
});
</script>
</body>''')
(dist/'index.html').write_text(html,encoding='utf-8')
(dist/'favicon.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="12" fill="#241934"/><path d="M12 44V31M22 49V21M32 54V10M42 49V21M52 44V31" stroke="#d9b567" stroke-width="5" stroke-linecap="round"/></svg>',encoding='utf-8')
for name in ['LICENSE-MIT','LICENSE-APACHE','NOTICE','ATTRIBUTION.md','ACA_FORK.json','ACA_FORK.md']:shutil.copyfile(ROOT/name,dist/name)
report={'application':'ACA Sound Studio','upstream':'https://github.com/storytold/soundcraft','upstreamCommit':json.loads((ROOT/'ACA_FORK.json').read_text())['upstreamCommit'],'sourceCommit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'wasmBindgen':version,'builtFromModifiedSource':True,'browserTested':False,'files':[{'path':str(f.relative_to(dist)).replace('\\','/'),'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(dist.rglob('*')) if f.is_file()]}
(ROOT/'ACA_WEB_BUILD.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'dist':str(dist),'wasmBytes':(dist/'soundcraft_web_bg.wasm').stat().st_size},indent=2))
