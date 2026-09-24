"""Fresh Linux build/tests in a network namespace, without runner-wide network changes.

This is direct IP-network isolation, not a hostile-build filesystem/IPC sandbox.
No fallback runs tests on the connected host when isolation cannot be established.
"""
from __future__ import annotations

import errno
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
STAGE = 'entry'


def stage(value: str) -> None:
    global STAGE
    STAGE = value
    print('Isolation stage: ' + value, flush=True)
COMMANDS = (
    ('tests', ['test', '--workspace', '--all-targets', '--no-fail-fast']),
    ('release_tests', ['test', '--workspace', '--all-targets', '--release', '--no-fail-fast']),
    ('doctests', ['test', '--workspace', '--doc', '--no-fail-fast']),
    ('clippy', ['clippy', '--workspace', '--all-targets']),
)
CAPABILITIES = ('CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb')


def validate_status(status: str, uid: int, parent_namespace: str, current_namespace: str) -> None:
    fields = dict(line.split(':', 1) for line in status.splitlines() if ':' in line)
    if (uid <= 0 or fields.get('NoNewPrivs', '').strip() != '1'
            or parent_namespace == current_namespace
            or not parent_namespace.startswith('net:[') or not current_namespace.startswith('net:[')):
        raise ValueError('Isolation or privilege boundary absent')
    if [int(v) for v in fields.get('Uid', '').split()] != [uid] * 4:
        raise ValueError('Unexpected process owner')
    if any(int(fields.get(field, '-1').strip(), 16) != 0 for field in CAPABILITIES):
        raise ValueError('Capabilities remain enabled')


def no_ip_route() -> None:
    # Query the current network namespace through the socket API, not a sysfs
    # mount inherited from the connected host. An errno alone is never proof.
    interfaces = socket.if_nameindex()
    if len(interfaces) != 1 or interfaces[0][1] != 'lo':
        raise ValueError('Isolated namespace must contain only loopback')
    # UDP connect checks numeric routing without sending a packet or querying DNS.
    for family, address in ((socket.AF_INET, ('192.0.2.1', 9)),
                            (socket.AF_INET6, ('2001:db8::1', 9))):
        try:
            with socket.socket(family, socket.SOCK_DGRAM) as probe:
                probe.connect(address)
        except OSError as error:
            refused = {errno.ENETUNREACH, errno.EHOSTUNREACH, errno.EAFNOSUPPORT}
            # A fresh namespace with no IPv6 source address can return this before
            # routing. Accept it only for IPv6 AND the independently empty topology.
            if family == socket.AF_INET6:
                refused.add(errno.EADDRNOTAVAIL)
            if error.errno in refused:
                continue
            print(json.dumps({'network_probe_family': int(family), 'inconclusive_errno': error.errno}), flush=True)
            raise ValueError('Network probe was inconclusive') from None
        raise ValueError('Unexpected route in isolated build')


def inner(config: dict) -> int:
    stage('verify-namespace-owner-and-capabilities')
    validate_status(Path('/proc/self/status').read_text(), config['uid'],
                    config['namespace'], os.readlink('/proc/self/ns/net'))
    stage('verify-no-ip-routes')
    no_ip_route()
    stage('verify-fresh-build-inputs')
    env = config['env']
    env.update({'CARGO_TARGET_DIR': config['target'], 'CARGO_NET_OFFLINE': 'true'})
    target = Path(config['target'])
    if not target.is_dir() or any(target.iterdir()):
        raise ValueError('Isolated build must start with an empty target directory')
    print('Only loopback present; IPv4/IPv6 probes refused; capabilities dropped; fresh target verified', flush=True)
    stage('execute-offline-checks')
    outcomes = {}
    for name, arguments in COMMANDS:
        command = [config['cargo'], *arguments, '--locked', '--offline']
        if name == 'clippy':
            command += ['--', '-D', 'warnings']
        print('Isolated check: ' + name, flush=True)
        try:
            result = subprocess.run(command, cwd=ROOT, env=env, timeout=480, check=False)
            outcomes[name] = 'success' if result.returncode == 0 else 'failure'
        except (OSError, subprocess.SubprocessError):
            outcomes[name] = 'failure'
    print(json.dumps({'network_isolated_checks': outcomes, 'qualification': 'not_established'}), flush=True)
    return 0 if set(outcomes.values()) == {'success'} else 1


def run() -> int:
    stage('verify-runner-and-utilities')
    if sys.platform != 'linux' or os.getuid() == 0:
        raise ValueError('Requires an ordinary standard Linux runner')
    # Reviewed acquisition is performed before this command. Do not acquire here.
    cargo = shutil.which('cargo')
    if not cargo:
        raise ValueError('Reviewed Cargo toolchain unavailable')
    paths = ['/usr/bin/sudo', '/usr/bin/unshare', '/usr/bin/setpriv', '/usr/bin/env']
    if any(not Path(path).is_file() for path in paths):
        raise ValueError('Standard runner isolation utilities unavailable')
    environment = {name: os.environ[name] for name in
                   ('HOME', 'PATH', 'RUSTUP_HOME', 'CARGO_HOME', 'LANG', 'TMPDIR') if name in os.environ}
    with tempfile.TemporaryDirectory(prefix='ca-isolated-', dir=os.environ['RUNNER_TEMP']) as target:
        config = {'uid': os.getuid(), 'namespace': os.readlink('/proc/self/ns/net'),
                  'cargo': cargo, 'target': target, 'env': environment}
        payload = json.dumps(config)
        if len(payload.encode()) > 16384:
            raise ValueError('Isolated launch inputs exceed their bound')
        # Only unshare/setpriv run with elevated privileges. Python and every
        # build/test process run under the original uid with empty capability sets.
        command = [paths[0], '-n', paths[1], '--net', '--', paths[2],
                   f'--reuid={os.getuid()}', f'--regid={os.getgid()}', '--clear-groups',
                   '--inh-caps=-all', '--ambient-caps=-all', '--bounding-set=-all',
                   '--no-new-privs', '--', paths[3], '-i', 'PATH=/usr/bin:/bin',
                   str(Path(sys.executable).resolve()), str(Path(__file__).resolve()), 'inner']
        stage('create-namespace-and-drop-privileges')
        result = subprocess.run(command, input=payload, text=True, cwd=ROOT, timeout=1500, check=False)
        return result.returncode


if __name__ == '__main__':
    try:
        if sys.argv[1:] == ['inner']:
            data = sys.stdin.read(16385)
            if len(data.encode()) > 16384:
                raise ValueError('Isolation input bound exceeded')
            result = inner(json.loads(data))
        elif not sys.argv[1:]:
            result = run()
        else:
            raise ValueError('Invalid invocation')
        raise SystemExit(result)
    except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError):
        raise SystemExit('Isolated build failed at ' + STAGE + '; no connected-host fallback was used') from None
