import socket
import time
import os
import subprocess

SOCK = "/tmp/qemu_ff_mon.sock"
LOG = "/tmp/qemu_ff.log"
OUT_DIR = "/home/inchara/.gemini/antigravity-cli/brain/48951f37-75ca-4df9-968a-a3c86972c48f"

for f in [SOCK, LOG]:
    if os.path.exists(f):
        os.remove(f)

cmd = [
    "./run.sh",
    "-display", "vnc=:96",
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
            if "Starting Leviathan Desktop Environment" in f.read():
                print("Desktop booted!")
                break
    time.sleep(0.5)

time.sleep(2.0)

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
capture(s, "ff_step1_boot")

# Step 2: Open All Apps launcher by clicking Dock 'All Apps' icon at bottom left
# Dock is centered at bottom. Total width: 1280.
# Dock items: All Apps, Terminal, Packages, Settings, Files, 3D Abyss
# Let's click Dock 'All Apps' at (x = 440, y = 770) or launch via terminal
# In tablet coordinates (0..32767):
# x = 440 -> int(440/1280 * 32767) = 11264
# y = 770 -> int(770/800 * 32767) = 31538
print("\n--- 2. Clicking All Apps in Dock ---")
mon_cmd(s, "mouse_move 11264 31538")
time.sleep(0.3)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(0.6)
capture(s, "ff_step2_launcher_open")

# In the All Apps launcher, Firefox is in the tiles (or click on Internet / search)
# Let's click on the tile for Firefox at (x=400, y=340) -> tab_x = int(400/1280*32767) = 10239, tab_y = int(340/800*32767) = 13926
# Or let's type 'firefox' into the launcher search box!
print("\n--- 3. Typing 'firefox' into search and pressing Enter ---")
for ch in "firefox":
    mon_cmd(s, f"sendkey {ch}")
    time.sleep(0.05)
time.sleep(0.5)
capture(s, "ff_step3_search_typed")

# Click first search result (Firefox tile at x=400, y=350)
mon_cmd(s, "mouse_move 10239 14335")
time.sleep(0.3)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
print("Clicked Firefox app card! Waiting 4s for Firefox to launch...")
time.sleep(4.0)

capture(s, "ff_step4_firefox_window")

# Step 4: Click Address bar in Firefox and type https://example.com
# Firefox window is spawned centered: x ≈ 260, y ≈ 160, w=760, h=440.
# Client rect begins at x=261, y=188.
# Address bar inside Firefox is around local x=280, local y=75.
# Screen coords: x = 261 + 280 = 541, y = 188 + 75 = 263.
# Tablet coords:
# x = int(541/1280*32767) = 13848
# y = int(263/800*32767) = 10772
print("\n--- 5. Clicking address bar in Firefox ---")
mon_cmd(s, "mouse_move 13848 10772")
time.sleep(0.3)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(0.5)

print("Typing URL 'https://example.com' into address bar...")
for ch in "https://example.com":
    if ch == ':':
        mon_cmd(s, "sendkey shift-semicolon")
    elif ch == '/':
        mon_cmd(s, "sendkey slash")
    elif ch == '.':
        mon_cmd(s, "sendkey dot")
    else:
        mon_cmd(s, f"sendkey {ch}")
    time.sleep(0.06)

time.sleep(0.2)
mon_cmd(s, "sendkey ret")
time.sleep(1.0)
capture(s, "ff_step5_url_entered")

# Step 5: Test Maximize button [+] / [□]
# Maximize button is at (win.x + win.width + 2 - 48 = 260 + 762 - 48 = 974, win.y + 4 = 164)
# Tablet coords:
# x = int(974 / 1280 * 32767) = 24933
# y = int(164 / 800 * 32767) = 6717
print("\n--- 6. Clicking Maximize button ---")
mon_cmd(s, "mouse_move 24933 6717")
time.sleep(0.3)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(1.5)
capture(s, "ff_step6_maximized")

# Step 6: Test Restore / Unmaximize
# When maximized: win.x = 0, win.y = 28, width = 1278.
# Maximize button is now at (0 + 1280 - 48 = 1232, 28 + 4 = 32)
# Tablet coords:
# x = int(1232 / 1280 * 32767) = 31538
# y = int(32 / 800 * 32767) = 1310
print("\n--- 7. Clicking Restore button to unmaximize ---")
mon_cmd(s, "mouse_move 31538 1310")
time.sleep(0.3)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(1.0)
capture(s, "ff_step7_restored")

# Step 7: Test Close [X] on Firefox
# Restored window close button at (260 + 762 - 24 = 998, 164)
# Tablet coords:
# x = int(998 / 1280 * 32767) = 25547
# y = int(164 / 800 * 32767) = 6717
print("\n--- 8. Clicking Close [X] on Firefox ---")
mon_cmd(s, "mouse_move 25547 6717")
time.sleep(0.3)
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(1.0)
capture(s, "ff_step8_closed")

mon_cmd(s, "quit")
s.close()
print("All Firefox WM integration tests completed!")
