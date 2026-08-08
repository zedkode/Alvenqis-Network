#!/usr/bin/env python3
"""Export the current VPS credentials to a local, gitignored Markdown file.

Secret values are transferred over the existing SSH connection and are never
written to stdout. The destination is created with owner-only permissions.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from datetime import datetime, timezone


REMOTE_EXPORT = r"""
import json
from pathlib import Path

workspace = Path('/opt/alvenqis/Blockchain-prototype/alvenqis-release/alvenqis-setup-external')
env_path = workspace / '.env'
secrets_path = Path('/var/lib/alvenqis/secrets')

environment = {}
for raw_line in env_path.read_text().splitlines():
    line = raw_line.strip()
    if not line or line.startswith('#') or '=' not in line:
        continue
    key, value = line.split('=', 1)
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        value = value[1:-1]
    environment[key.strip()] = value

secrets = {}
for path in sorted(secrets_path.iterdir()):
    if path.is_file():
        secrets[path.name] = path.read_text().strip()

print(json.dumps({'environment': environment, 'secrets': secrets}))
"""


def fenced(value: str) -> str:
    return f"```text\n{value}\n```"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--host', default='144.91.81.81')
    parser.add_argument('--user', default='root')
    parser.add_argument(
        '--identity',
        default=str(Path.home() / '.ssh' / 'id_ed25519_alvenqis_vps'),
    )
    parser.add_argument('--output', default='credentials.md')
    args = parser.parse_args()

    identity = Path(args.identity).expanduser().resolve()
    if not identity.is_file():
        raise SystemExit(f'SSH identity not found: {identity}')

    command = [
        'ssh',
        '-i',
        str(identity),
        '-o',
        'BatchMode=yes',
        '-o',
        'StrictHostKeyChecking=yes',
        f'{args.user}@{args.host}',
        'python3 -',
    ]
    completed = subprocess.run(
        command,
        input=REMOTE_EXPORT,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        print('Credential export failed over SSH.', file=sys.stderr)
        return completed.returncode

    payload = json.loads(completed.stdout)
    environment: dict[str, str] = payload['environment']
    secrets: dict[str, str] = payload['secrets']

    host = args.host
    ssh_target = f'{args.user}@{host}'
    generated = datetime.now(timezone.utc).isoformat(timespec='seconds')
    lines = [
        '# Alvenqis VPS credentials',
        '',
        f'Generated from the live VPS on `{generated}`.',
        '',
        '> Sensitive local file. Owner-readable only (`0600`) and excluded from Git.',
        '',
        '## SSH',
        '',
        f'- Host: `{host}`',
        '- Port: `22`',
        f'- Username: `{args.user}`',
        '- Authentication: SSH key (no password)',
        f'- Identity file: `{identity}`',
        f'- Ready command: `ssh -i {identity} {ssh_target}`',
        '',
        '## Human-facing logins',
        '',
    ]

    login_specs = [
        ('Control operator', 'CONTROL_HOST', 'ADMIN_OPERATOR_USER', 'admin_password'),
        ('Control viewer', 'CONTROL_HOST', 'ADMIN_VIEWER_USER', 'admin_viewer_password'),
        ('Grafana administrator', 'GRAFANA_HOST', 'GRAFANA_ADMIN_USER', 'grafana_password'),
    ]
    for label, host_key, user_key, secret_key in login_specs:
        lines.extend(
            [
                f'### {label}',
                '',
                f'- URL: `https://{environment.get(host_key, "")}`',
                f'- Username: `{environment.get(user_key, "")}`',
                '- Password:',
                '',
                fenced(secrets.get(secret_key, '')),
                '',
            ]
        )

    lines.extend(['## Service and provisioning credentials', ''])
    for name in sorted(secrets):
        lines.extend([f'### `{name}`', '', fenced(secrets[name]), ''])

    lines.extend(['## Public service hosts', ''])
    for key in (
        'WEBSITE_HOST',
        'WWW_HOST',
        'EXPLORER_HOST',
        'CONTROL_HOST',
        'PUBLIC_RPC_HOST',
        'POOL_HOST',
        'GRAFANA_HOST',
        'PROMETHEUS_HOST',
        'FLEET_HOST',
        'FLEET_MTLS_HOST',
        'P2P_HOST',
        'STRATUM_HOST',
    ):
        if environment.get(key):
            lines.append(f'- `{key}`: `{environment[key]}`')
    lines.append('')

    output = Path(args.output).resolve()
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    descriptor = os.open(output, flags, 0o600)
    try:
        with os.fdopen(descriptor, 'w', encoding='utf-8') as handle:
            handle.write('\n'.join(lines))
    except Exception:
        output.unlink(missing_ok=True)
        raise
    os.chmod(output, 0o600)
    print(f'Wrote {output} with mode 0600 ({len(secrets)} secret fields).')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
