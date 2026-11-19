"""Read macOS console availability without interacting with the desktop."""
from enum import IntEnum
import plistlib
import subprocess
from xml.parsers.expat import ExpatError


class DesktopStatus(IntEnum):
    READY = 0
    LOCKED = 3
    UNAVAILABLE = 4


DEFERRED_DESKTOP_EXITS = {
    DesktopStatus.LOCKED: "macOS desktop is locked",
    DesktopStatus.UNAVAILABLE: "macOS desktop is unavailable or its state could not be determined",
}


def console_users(nodes):
    if not isinstance(nodes, list):
        return
    for node in nodes:
        if not isinstance(node, dict):
            continue
        users = node.get("IOConsoleUsers")
        if isinstance(users, list):
            yield from users
        yield from console_users(node.get("IORegistryEntryChildren"))


def parse_console_sessions(payload):
    try:
        registry = plistlib.loads(payload)
    except (plistlib.InvalidFileException, ExpatError, ValueError, IndexError, AttributeError):
        return DesktopStatus.UNAVAILABLE

    active = [user for user in console_users(registry) if isinstance(user, dict)
              and user.get("kCGSSessionOnConsoleKey") is True
              and user.get("kCGSessionLoginDoneKey") is True]
    if len(active) != 1:
        return DesktopStatus.UNAVAILABLE
    # macOS can omit the lock property when the active session is unlocked.
    locked = active[0].get("CGSSessionScreenIsLocked", False)
    if locked is True:
        return DesktopStatus.LOCKED
    if locked is False:
        return DesktopStatus.READY
    return DesktopStatus.UNAVAILABLE


def desktop_status():
    try:
        result = subprocess.run(["/usr/sbin/ioreg", "-a", "-n", "Root", "-d", "1"],
                                capture_output=True, check=True, timeout=3)
    except (OSError, subprocess.SubprocessError):
        return DesktopStatus.UNAVAILABLE
    return parse_console_sessions(result.stdout)
