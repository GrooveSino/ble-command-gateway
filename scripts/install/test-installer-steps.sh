#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
python3 - "$ROOT_DIR" <<'PY'
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

root = Path(sys.argv[1])
with tempfile.TemporaryDirectory() as temporary:
    base = Path(temporary)
    for mode in ('tui', 'plain'):
        for case, task, expected in (
            ('quick', 'echo task-output; echo once >> "$CASE_DIR/count"', 0),
            ('delayed', 'sleep 0.1; echo once >> "$CASE_DIR/count"', 0),
            ('failure', 'echo diagnostic >&2; return 7', 7),
            ('intermediate', 'echo diagnostic >&2; false; echo MASKED', 1),
            ('pipeline', 'echo diagnostic >&2; false | cat; echo MASKED', 1),
        ):
            directory = base / (mode + '-' + case)
            directory.mkdir()
            logs = directory / 'logs'
            logs.mkdir()
            script = '''set -Eeuo pipefail
source "$ROOT/scripts/install/installer/lib/tui.sh"
# Any rendering attempt fails: step execution must be independent of Gum.
gum() { echo GUM_CALLED >&2; return 99; }
tui_ready() { [ "$MODE" = tui ]; }
task() { TASK_BODY; }
tui_run_step test task
echo NEXT
[[ $- == *e* ]] || exit 88
'''.replace('TASK_BODY', task)
            env = dict(os.environ, ROOT=str(root), MODE=mode, CASE_DIR=str(directory), TMPDIR=str(logs))
            result = subprocess.run(['bash', '-c', script], env=env, capture_output=True, text=True, timeout=5)
            output = result.stdout + result.stderr
            assert result.returncode == expected, (mode, case, result)
            assert ('NEXT' in output) == (expected == 0), output
            assert 'GUM_CALLED' not in output and 'MASKED' not in output, output
            if expected:
                assert 'diagnostic' in output and '步骤失败' in output, output
            else:
                assert (directory / 'count').read_text() == 'once\n'
                assert '完成' in output, output
            if mode == 'plain' and case == 'quick':
                assert 'task-output' in output, output
            assert not list(logs.iterdir()), list(logs.iterdir())

        for sig, expected in ((signal.SIGINT, 130), (signal.SIGTERM, 143)):
            directory = base / (mode + '-' + sig.name)
            directory.mkdir()
            logs = directory / 'logs'
            logs.mkdir()
            env = dict(os.environ, ROOT=str(root), MODE=mode, CASE_DIR=str(directory), TMPDIR=str(logs))
            script = '''set -Eeuo pipefail
source "$ROOT/scripts/install/installer/lib/tui.sh"
tui_ready() { [ "$MODE" = tui ]; }
task() { touch "$CASE_DIR/ready"; sleep 30; touch "$CASE_DIR/task-finished"; }
tui_run_step interrupted task
touch "$CASE_DIR/next"
'''
            process = subprocess.Popen(['bash', '-c', script], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                deadline = time.monotonic() + 5
                while not (directory / 'ready').exists():
                    assert time.monotonic() < deadline, 'task never started'
                    time.sleep(0.01)
                os.killpg(process.pid, sig)
                stdout, stderr = process.communicate(timeout=5)
                assert process.returncode in (expected, -sig), (process.returncode, stdout, stderr)
                assert not (directory / 'next').exists()
                assert not (directory / 'task-finished').exists()
                assert '✓' not in stdout
                assert not list(logs.iterdir()), list(logs.iterdir())
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.communicate()
    # Exercise the real install orchestrator and archive/activation helpers without
    # sudo, systemd, downloads, or Bluetooth hardware.
    integration = base / 'integration'
    integration.mkdir()
    import tarfile
    package = integration / 'package'
    package.mkdir()
    (package / 'yundrone-ble-server').write_text('#!/bin/sh\nexit 0\n')
    with tarfile.open(integration / 'fixture.tar.gz', 'w:gz') as archive:
        archive.add(package / 'yundrone-ble-server', arcname='yundrone-ble-server')
    script = r'''set -Eeuo pipefail
source "$ROOT/scripts/install/installer/lib/tui.sh"
source "$ROOT/scripts/install/installer/lib/release.sh"
source "$ROOT/scripts/install/installer/commands/core.sh"
INSTALL_ROOT="$CASE_DIR/install"
LEGACY_IDENTITY_FILE="$CASE_DIR/legacy-name"
PREFIX=yundrone BACKEND=auto VERSION=latest SERVICE_NAME=test.service
TUI_LOADING=no
debug() { :; }
info() { echo "$*"; }
warn() { echo "$*" >&2; }
fail() { echo "$*" >&2; exit 1; }
section() { :; }
strong() { echo "$*"; }
field_line() { :; }
version_text() { echo "$*"; }
accent() { echo "$*"; }
path_text() { echo "$*"; }
good_text() { echo "$*"; }
muted() { echo "$*"; }
tui_ready() { return 1; }
validate_prefix() { :; }
detect_arch() { echo linux-arm64; }
adapter_display() { echo mock; }
identity_name() { echo yundrone-bleinit-123456; }
identity_serial() { echo 123456; }
render_identity_reminder() { :; }
confirm_yes() { return 0; }
confirm_no() { return 1; }
ensure_sudo_step() { :; }
maybe_install_dependencies() { :; }
ensure_basic_tools() { :; }
ensure_python() { :; }
run_root() { "$@"; }
resolve_release_asset() { echo fixture > "$2/selected-version"; }
download_release_tarball() { cp "$CASE_DIR/fixture.tar.gz" "$1/server.tar.gz"; }
verify_release_tarball() { :; }
stop_service_for_install() { echo stop >> "$CASE_DIR/order"; }
write_service() {
  test -x "$INSTALL_ROOT/current/yundrone-ble-server"
  echo write >> "$CASE_DIR/order"
}
systemctl() { echo "$1" >> "$CASE_DIR/order"; }
enable_service() { echo enable >> "$CASE_DIR/order"; }
start_or_restart_service() { echo start >> "$CASE_DIR/order"; }
wait_service_active() { :; }
wait_identity_ready() { :; }
render_install_success() { echo success >> "$CASE_DIR/order"; }
install_or_update
'''
    result = subprocess.run(['bash', '-c', script], env=dict(os.environ, ROOT=str(root), CASE_DIR=str(integration)), capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result
    assert (integration / 'order').read_text().splitlines() == ['stop', 'write', 'daemon-reload', 'enable', 'start', 'success']
print('installer step tests passed (success, failures, strict mode, Gum unavailable, signals; TUI/plain)')
PY
