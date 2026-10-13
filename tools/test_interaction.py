import socket
import time
import os
import subprocess

PPM_DIR = "/tmp"
OUT_DIR = "/home/inchara/.gemini/antigravity-cli/brain/48951f37-75ca-4df9-968a-a3c86972c48f"
SOCK = "/tmp/qemu_test_mon.sock"

def mon_cmd(sock, cmd):
    sock.sendall(cmd.encode() + b"\n")
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
    else:
        print(f"Failed to capture: {ppm}")
        return None

# Connect to monitor
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(SOCK)
s.settimeout(2.0)
time.sleep(0.2)
try:
    s.recv(1024)
except Exception:
    pass

print("1. Initial state screenshot")
capture(s, "step1_initial")

# In our screenshot, cursor starts at center (640, 400).
# The Test Window is at x=160, y=100, width=420, height=220.
# Full window rect is x: 160..582, y: 100..349.
# Titlebar is x: 160..582, y: 100..128.
# Close button is x: 558..578, y: 104..124.

# Let's drag titlebar: move mouse from center (640, 400) to titlebar (300, 114):
# dx = 300 - 640 = -340, dy = 114 - 400 = -286.
print("2. Moving mouse to titlebar...")
mon_cmd(s, "mouse_move -340 -286")
time.sleep(0.3)
capture(s, "step2_hover_titlebar")

print("3. Mouse press left button...")
mon_cmd(s, "mouse_button 1")
time.sleep(0.3)

print("4. Drag window to the right and down: dx=200, dy=150...")
mon_cmd(s, "mouse_move 200 150")
time.sleep(0.3)
capture(s, "step3_dragged_window")

print("5. Mouse release...")
mon_cmd(s, "mouse_button 0")
time.sleep(0.3)
capture(s, "step4_released_drag")

# Now window was at (160, 100) and moved by (+200, +150) -> new position is approx (360, 250).
# Let's move mouse to its new close button and click it!
# Current mouse pos is (300 + 200, 114 + 150) = (500, 264).
# New close button is at (360 + 422 - 24, 250 + 4) = (758, 254).
# dx = 758 - 500 = 258, dy = 254 - 264 = -10.
print("6. Moving mouse to close button [X]...")
mon_cmd(s, "mouse_move 258 -10")
time.sleep(0.3)
capture(s, "step5_hover_close")

print("7. Clicking [X] to close window...")
mon_cmd(s, "mouse_button 1")
time.sleep(0.2)
mon_cmd(s, "mouse_button 0")
time.sleep(0.5)
capture(s, "step6_window_closed")

s.sendall(b"quit\n")
s.close()
print("Test completed successfully!")
