from __future__ import annotations
import json, os, secrets, shlex, subprocess, threading, time
from pathlib import Path
from flask import Flask, jsonify, request
app=Flask(__name__)
WORKSPACE=Path(os.environ['ALVENQIS_WORKSPACE']).resolve(); COMPOSE_FILE=Path(os.environ.get('ALVENQIS_COMPOSE_FILE',WORKSPACE/'compose.yaml')); TOKEN_FILE=Path(os.environ.get('BROKER_TOKEN_FILE','/run/secrets/broker_token'))
def token(): return TOKEN_FILE.read_text().strip()
def auth():
 v=request.headers.get('X-Alvenqis-Broker-Token',''); return bool(v and secrets.compare_digest(v,token()))
def load_env():
 env=os.environ.copy(); p=WORKSPACE/'.env'
 if p.exists():
  for raw in p.read_text().splitlines():
   line=raw.strip()
   if not line or line.startswith('#') or '=' not in line: continue
   k,v=line.split('=',1)
   try: parts=shlex.split(v); env[k]=parts[0] if parts else ''
   except ValueError: env[k]=v.strip().strip("'\"")
 for k in ('ALVENQIS_HOST_WORKSPACE','ALVENQIS_HOST_REPO','ALVENQIS_COMPOSE_FILE'):
  if k in os.environ: env[k]=os.environ[k]
 return env
def cfg(): return load_env()
def compose(*args, profiles=()):
 e=cfg(); cmd=['docker','compose','--env-file',str(WORKSPACE/'.env'),'-f',str(COMPOSE_FILE)]
 if e.get('CLOUDFLARE_MODE','disabled')!='tunnel': cmd += ['-f',str(WORKSPACE/'compose.direct.yaml')]
 for p in profiles: cmd += ['--profile',p]
 return cmd+list(args)
def run(args,timeout=7200,check=True):
 r=subprocess.run(args,cwd=WORKSPACE,env=cfg(),text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=timeout)
 if check and r.returncode: raise RuntimeError(f"command failed ({r.returncode}): {' '.join(args)}\n{r.stdout}")
 return r.stdout
def profiles():
 e=cfg(); p=['backup']
 if e.get('CLOUDFLARE_MODE')=='tunnel': p.append('cloudflare')
 if e.get('ENABLE_POOL','false').lower()=='true': p.append('pool')
 return p
def schedule_installer_stop():
 if os.environ.get('ALVENQIS_INSTALLER_MODE','false').lower()!='true': return
 def stop_later():
  time.sleep(10)
  subprocess.run(['docker','stop','alvenqis-installer'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=False)
  subprocess.run(['docker','stop','alvenqis-installer-broker'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=False)
 threading.Thread(target=stop_later,daemon=True).start()
def deploy():
 e=cfg(); out=[run(['bash',str(WORKSPACE/'scripts/prepare-state.sh')],120),run(['bash',str(WORKSPACE/'scripts/runtime-preflight.sh')],120),run(compose('config'),120)]
 if e.get('CLOUDFLARE_MODE','disabled')=='tunnel': out.append(run(['bash',str(WORKSPACE/'scripts/cloudflare-bootstrap.sh'),'--prepare'],600))
 # Deliberately build from the checked-out repository. No pull, updater, mutable tag refresh or scheduled image replacement.
 args=('up','-d','--build')
 out.append(run(compose(*args,profiles=profiles()),7200)); out.append(run(['bash',str(WORKSPACE/'scripts/health-check-docker.sh')],600))
 if e.get('CLOUDFLARE_MODE','disabled')!='disabled':
  out.append(run(['bash',str(WORKSPACE/'scripts/cloudflare-bootstrap.sh'),'--activate'],600))
  out.append(run(['bash',str(WORKSPACE/'scripts/verify-public-health.sh')],300))
 schedule_installer_stop()
 return '\n'.join(out)
def status():
 raw=run(compose('ps','--format','json'),120,False); services=[]
 for line in raw.splitlines():
  try: services.append(json.loads(line))
  except json.JSONDecodeError: pass
 return {'configured':(WORKSPACE/'.env').exists(),'services':services,'raw':raw}
@app.get('/health')
def health(): return jsonify({'ok':True,'service':'alvenqis-docker-broker'})
@app.post('/v1/action')
def action():
 if not auth(): return jsonify({'error':'unauthorized'}),401
 p=request.get_json(silent=True) or {}; a=str(p.get('action',''))
 try:
  if a=='deploy': return jsonify({'ok':True,'output':deploy()})
  if a=='status': return jsonify({'ok':True,**status()})
  if a=='backup': return jsonify({'ok':True,'output':run(['bash',str(WORKSPACE/'scripts/backup-now.sh')])})
  return jsonify({'error':'unsupported action'}),400
 except (OSError,RuntimeError,ValueError,subprocess.TimeoutExpired) as ex: return jsonify({'error':f'{type(ex).__name__}: {ex}'}),500
