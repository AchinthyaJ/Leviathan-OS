import socket
import time
import os
import subprocess

SOCK = "/tmp/qemu_native_mon.sock"
LOG = "/tmp/qemu_native_serial.log"
OUT_DIR = "/home/inchara/.gemini/antigravity-cli/brain/48951f37-75ca-4df9-968a-a3c86972c48f"

for f in [SOCK, LOG]:
    if os.path.exists(f):
        os.remove(f)

cmd = [
    "./run.sh",
    "-display", "vnc=:98",
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
    time.sleep(0.8)
    if os.path.exists(ppm):
        subprocess.run(["convert", ppm, png], check=True)
        print(f"Captured: {png}")
        return png
    return None

# Wait for boot
print("Waiting for desktop...")
for i in range(40):
    if os.path.exists(LOG):
        with open(LOG, "r", errors="ignore") as f:
            content = f.read()
            if "Starting Leviathan Desktop Environment" in content:
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

# Step 1: Initial Desktop
print("\n--- 1. Initial Desktop ---")
capture(s, "native_step1_desktop")

# Step 2: Open Terminal by clicking Terminal on Dock (x=520, y=756)
# Tablet coordinates: (13312, 30965)
print("\n--- 2. Clicking Terminal on Dock ---")
mon_cmd(s, "mouse_move 13312 30965")
time.sleep(0.2)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(0.8)
capture(s, "native_step2_terminal_open")

# Step 3: Type 'xclock' into Terminal and press Enter
print("\n--- 3. Launching xclock (real installed .deb) from Terminal ---")
for ch in "xclock":
    mon_cmd(s, f"sendkey {ch}")
    time.sleep(0.05)
mon_cmd(s, "sendkey ret")
time.sleep(2.5)

capture(s, "native_step3_xclock_running")

# Step 4: Check serial log
print("\n--- Serial Log (last 1500 chars) ---")
if os.path.exists(LOG):
    with open(LOG, "r", errors="ignore") as f:
        print(f.read()[-1500:])

s.close()
