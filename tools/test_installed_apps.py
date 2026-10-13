import socket
import time
import os
import subprocess

SOCK = "/tmp/qemu_apps_mon.sock"
LOG = "/tmp/qemu_apps_serial.log"
OUT_DIR = "/home/inchara/.gemini/antigravity-cli/brain/48951f37-75ca-4df9-968a-a3c86972c48f"

for f in [SOCK, LOG]:
    if os.path.exists(f):
        os.remove(f)

cmd = [
    "./run.sh",
    "-display", "vnc=:97",
    "-monitor", f"unix:{SOCK},server,nowait",
    "-serial", f"file:{LOG}",
    "-daemonize"
]
print("Starting QEMU...")
subprocess.run(cmd, check=True)

def mon_cmd(sock, c):
    sock.sendall(c.encode() + b"\n")
    time.sleep(0.3)
    resp = b""
    while True:
        try:
            chunk = sock.recv(4096)
            if not chunk:
                break
            resp += chunk
            if b"(qemu)" in resp:
                break
        except Exception:
            break
    return resp.decode(errors="ignore")

def capture(sock, name):
    ppm = f"/tmp/{name}.ppm"
    png = f"{OUT_DIR}/{name}.png"
    if os.path.exists(ppm):
        os.remove(ppm)
    sock.sendall(f"screendump {ppm}\n".encode())
    time.sleep(0.6)
    if os.path.exists(ppm):
        subprocess.run(["convert", ppm, png], check=True)
        print(f"Captured: {png}")
        return png
    return None

# Wait for boot
print("Waiting for desktop...")
for i in range(50):
    if os.path.exists(LOG):
        with open(LOG, "r", errors="ignore") as f:
            content = f.read()
            if "Starting Leviathan Desktop Environment" in content or "Leviathan Rust UI" in content:
                print("Desktop booted!")
                break
    time.sleep(0.5)

time.sleep(3.0)

s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(SOCK)
s.settimeout(2.0)
time.sleep(0.3)
try:
    s.recv(1024)
except Exception:
    pass

# Step 1: Initial boot screenshot
print("\n--- 1. Initial Desktop ---")
capture(s, "app_step1_boot")

# Step 2: Open All Apps launcher by clicking Dock 'All Apps' icon at bottom
# Dock is centered at bottom of 1280x800 screen.
# All Apps icon is at (x = 450, y = 770). In virtio tablet (0..32767):
# x = 450 -> int(450/1280 * 32767) = 11520
# y = 770 -> int(770/800 * 32767) = 31538
print("\n--- 2. Clicking All Apps in Dock ---")
mon_cmd(s, "mouse_move 11520 31538")
time.sleep(0.3)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(0.8)
capture(s, "app_step2_launcher_open")

# Step 3: Click Terminal or search in All Apps
print("\n--- Checking serial log for scanned apps ---")
if os.path.exists(LOG):
    with open(LOG, "r", errors="ignore") as f:
        print(f.read()[-2000:])

s.close()
