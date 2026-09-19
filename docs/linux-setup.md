# Linux GUI Automation Setup & Diagnostic Guide (Ubuntu 22.04 LTS / GNOME)

This guide documents the prerequisites, configuration steps, and troubleshooting procedures for running **pain ai** GUI automation on Linux via AT-SPI and X11/Wayland.

---

## 1. Enable GNOME Accessibility Toolkit (AT-SPI)

AT-SPI relies on the D-Bus service `org.a11y.Bus` to inspect application element trees, find interactive nodes, and invoke native accessibility patterns (`click`, `press`, `select`, `activate`, `set-text-contents`).

### Step 1: Check Current Status
```bash
gsettings get org.gnome.desktop.interface toolkit-accessibility
```

- If output is `'true'`: Accessibility is already enabled.
- If output is `'false'`: Proceed to Step 2.

### Step 2: Enable Accessibility
```bash
gsettings set org.gnome.desktop.interface toolkit-accessibility true
```

### Step 3: Logout and Relogin (Mandatory)
> [!IMPORTANT]
> A full session logout and relogin (or system reboot) is **mandatory** after changing `toolkit-accessibility`. Running applications only register with the accessibility bus when launched within an active accessibility-enabled session.

---

## 2. Session Type Selection: X11 vs. Wayland

pain ai provides first-class native input on **X11**, with guided support for **Wayland**.

### Recommended: Select X11 on GDM Login
1. Log out of your current session.
2. On the GDM login screen, click your username.
3. Click the small **gear icon (⚙)** in the bottom right corner of the screen.
4. Select **"Ubuntu on Xorg"** (or **"GNOME on Xorg"**).
5. Enter your password and log in.

### Verify Current Session Type
```bash
echo $XDG_SESSION_TYPE
```
- Should return: `x11`

---

## 3. Wayland Synthetic Input Setup (`ydotool`)

On Wayland sessions, compositors prevent unprivileged processes from injecting global synthetic mouse or keyboard events. pain ai uses `ydotool` (via Linux `/dev/uinput`) for Wayland coordinate clicks and typing.

If pain ai detects a Wayland session without `ydotool`, it returns error code `WAYLAND_INPUT` with a link to this guide instead of failing silently.

### Install and Enable `ydotool`
```bash
# 1. Install ydotool from official Ubuntu repositories
sudo apt update && sudo apt install -y ydotool

# 2. Add your user to the input group for uinput permissions
sudo usermod -aG input $USER

# 3. Enable and start the background ydotoold user daemon
systemctl --user enable --now ydotoold
```

### Optional: Manual Daemon Startup
If your system does not run user systemd services, run `ydotoold` manually in a background terminal:
```bash
sudo ydotoold --socket-path=/tmp/.ydotool_socket &
export YDOTOOL_SOCKET=/tmp/.ydotool_socket
```

---

## 4. X11 vs. Wayland Capability Matrix

| Feature Layer | GNOME on X11 (Default / Recommended) | GNOME on Wayland (With `ydotoold`) | Wayland Without `ydotoold` |
|---|---|---|---|
| **Element Tree (`ui_tree`, `ui_find`)** | Full AT-SPI D-Bus tree inspection | Full AT-SPI D-Bus tree inspection | Full AT-SPI D-Bus tree inspection |
| **Native Actions (`ui_act`)** | Full (`invoke`, `set_value`, `toggle`, `focus`) | Full (`invoke`, `set_value`, `toggle`, `focus`) | Full (`invoke`, `set_value`, `toggle`, `focus`) |
| **Coordinate Click (`ui_click`)** | Native direct X11 click | `ydotool` simulated mouse click | Blocked (`WAYLAND_INPUT` guided error) |
| **Text Typing (`ui_type`)** | Native direct X11 keyboard | `ydotool` simulated keystrokes | Blocked (`WAYLAND_INPUT` guided error) |
| **Screen Capture (`ui_capture`)** | Direct primary monitor grab via `xcap` | Screen capture via portal / `xcap` | Screen capture via portal / `xcap` |
| **Clipboard Access** | Direct X11 clipboard selection | Wayland data-control / clipboard portal | Wayland data-control / clipboard portal |

---

## 5. Verification & Smoke Testing

### AT-SPI D-Bus Python Smoke Test
Verify that AT-SPI D-Bus communication is active and detecting application desktops:
```bash
python3 -c "import gi; gi.require_version('Atspi', '2.0'); from gi.repository import Atspi; print('AT-SPI Desktop Count:', Atspi.get_desktop_count())"
```
- Expected output: `AT-SPI Desktop Count: 1` (or greater)

### Interactive Tree Inspector (`accerciser`)
To visually inspect accessibility nodes, roles, and actions alongside pain ai:
```bash
sudo apt install -y accerciser
accerciser &
```

### Automated Diagnostic CLI (`lsc doctor`)
Run the integrated pain ai diagnostics suite:
```bash
lsc doctor
```
Checks:
- `[✓] AT-SPI D-Bus reachability`
- `[✓] GNOME toolkit-accessibility status`
- `[✓] Session type detection (X11 / Wayland)`
- `[✓] Input daemon status (native / ydotool)`
- `[✓] Screen capture subsystem`
