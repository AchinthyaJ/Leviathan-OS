#!/bin/bash
set -e

echo "========================================================="
echo "   Leviathan OS - Official Base Bootstrap (Ubuntu 24.04)"
echo "========================================================="
echo ""

if [ "$EUID" -ne 0 ]; then
  echo "ERROR: Please run this script with sudo:"
  echo "  sudo bash $0"
  exit 1
fi

PROJECT_DIR="/home/inchara/My Death/Leviathan"
ROOTFS="$PROJECT_DIR/rootfs"
BUILD_DIR="/tmp/leviathan-bootstrap"

echo ">>> Step 1/7: Installing debootstrap on host..."
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y debootstrap

echo ">>> Step 2/7: Bootstrapping pristine Ubuntu 24.04 (Noble) base..."
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

debootstrap \
    --variant=minbase \
    --arch=amd64 \
    --components=main,universe,restricted \
    --include=apt,ca-certificates,sudo,whiptail,dialog,xvfb,xdotool \
    noble "$BUILD_DIR" http://archive.ubuntu.com/ubuntu/

echo ">>> Step 3/7: Configuring system, user accounts & networking..."
# Set hostname & hosts
echo "leviathan-os" > "$BUILD_DIR/etc/hostname"
cat << 'EOF' > "$BUILD_DIR/etc/hosts"
127.0.0.1 localhost
127.0.1.1 leviathan-os
EOF

# Set DNS fallback
cat << 'EOF' > "$BUILD_DIR/etc/resolv.conf"
nameserver 8.8.8.8
nameserver 1.1.1.1
EOF

# Set Leviathan OS release identification
cat << 'EOF' > "$BUILD_DIR/usr/lib/os-release"
NAME="Leviathan OS"
PRETTY_NAME="Leviathan OS"
VERSION_ID="1.0"
VERSION="1.0"
VERSION_CODENAME=noble
ID=leviathan
ID_LIKE="ubuntu debian"
HOME_URL="https://github.com/AchinthyaJ/Leviathan-OS"
SUPPORT_URL="https://github.com/AchinthyaJ/Leviathan-OS"
BUG_REPORT_URL="https://github.com/AchinthyaJ/Leviathan-OS/issues"
PRIVACY_POLICY_URL="https://github.com/AchinthyaJ/Leviathan-OS"
UBUNTU_CODENAME=noble
LOGO=leviathan
EOF

cat << 'EOF' > "$BUILD_DIR/etc/lsb-release"
DISTRIB_ID=Leviathan
DISTRIB_RELEASE=1.0
DISTRIB_CODENAME=noble
DISTRIB_DESCRIPTION="Leviathan OS"
EOF

echo -e "Leviathan OS 1.0 \\n \\l\n" > "$BUILD_DIR/etc/issue"
echo "Leviathan OS 1.0" > "$BUILD_DIR/etc/issue.net"


# Configure official apt repositories (main, restricted, universe, multiverse, updates, security, backports)
cat << 'EOF' > "$BUILD_DIR/etc/apt/sources.list"
deb http://archive.ubuntu.com/ubuntu noble main restricted universe multiverse
deb http://archive.ubuntu.com/ubuntu noble-updates main restricted universe multiverse
deb http://security.ubuntu.com/ubuntu noble-security main restricted universe multiverse
deb http://archive.ubuntu.com/ubuntu noble-backports main restricted universe multiverse
EOF

# Configure non-interactive & robust APT defaults
cat << 'EOF' > "$BUILD_DIR/etc/apt/apt.conf.d/99leviathan.conf"
Apt::Cmd::Disable-Script-Warning "true";
APT::Get::Assume-Yes "true";
APT::Get::AllowUnauthenticated "true";
APT::Architecture "amd64";
APT::Architectures { "amd64"; };
Dpkg::Options { "--force-confdef"; "--force-confold"; };
EOF

# Configure Mozilla official APT repo & signing key for native .deb Firefox
mkdir -p "$BUILD_DIR/etc/apt/keyrings"
curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg > "$BUILD_DIR/etc/apt/keyrings/packages.mozilla.org.asc"

cat << 'EOF' > "$BUILD_DIR/etc/apt/sources.list.d/mozilla.list"
deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main
EOF

mkdir -p "$BUILD_DIR/etc/apt/preferences.d"
cat << 'EOF' > "$BUILD_DIR/etc/apt/preferences.d/mozilla"
Package: *
Pin: origin packages.mozilla.org
Pin-Priority: 1000
EOF

# Ensure DNS resolution works inside chroot
cp /etc/resolv.conf "$BUILD_DIR/etc/resolv.conf"

# Update package lists and pre-install native Firefox
echo "    Updating package lists and pre-installing native Firefox..."
chroot "$BUILD_DIR" apt-get update
chroot "$BUILD_DIR" apt-get install -y --no-install-recommends firefox

# Create user leviathan with passwordless sudo
chroot "$BUILD_DIR" useradd -m -s /bin/bash -u 1000 -G sudo leviathan 2>/dev/null || true
echo "leviathan:leviathan" | chroot "$BUILD_DIR" chpasswd 2>/dev/null || true
echo "root:root" | chroot "$BUILD_DIR" chpasswd 2>/dev/null || true
echo "leviathan ALL=(ALL) NOPASSWD: ALL" > "$BUILD_DIR/etc/sudoers.d/leviathan"
chmod 0440 "$BUILD_DIR/etc/sudoers.d/leviathan"

echo ">>> Step 4/7: Migrating Leviathan OS components & drivers..."
# Copy kernel modules & firmware
if [ -d "$ROOTFS/lib/modules" ]; then
    mkdir -p "$BUILD_DIR/lib/modules"
    cp -a "$ROOTFS/lib/modules/." "$BUILD_DIR/lib/modules/"
fi
if [ -d "$ROOTFS/lib/firmware" ]; then
    mkdir -p "$BUILD_DIR/lib/firmware"
    cp -a "$ROOTFS/lib/firmware/." "$BUILD_DIR/lib/firmware/"
fi

# Copy udhcpc script
if [ -d "$ROOTFS/usr/share/udhcpc" ]; then
    mkdir -p "$BUILD_DIR/usr/share/udhcpc"
    cp -a "$ROOTFS/usr/share/udhcpc/." "$BUILD_DIR/usr/share/udhcpc/"
fi

# Copy Leviathan boot init script
cp -a "$ROOTFS/init" "$BUILD_DIR/init"
chmod +x "$BUILD_DIR/init"

# Copy Leviathan binaries
for bin in leviathan-ui leviathan busybox getty fbdiag first_pixel leviathan_art; do
    if [ -f "$ROOTFS/bin/$bin" ]; then
        cp -a "$ROOTFS/bin/$bin" "$BUILD_DIR/bin/$bin"
        chmod +x "$BUILD_DIR/bin/$bin"
    fi
done

# Ensure busybox symlinks exist in /bin
if [ -f "$BUILD_DIR/bin/busybox" ]; then
    chroot "$BUILD_DIR" /bin/busybox --install -s /bin 2>/dev/null || true
fi

# Install dpkg-divert safety wrapper
if [ -f "$BUILD_DIR/usr/bin/dpkg-divert" ] && [ ! -f "$BUILD_DIR/usr/bin/dpkg-divert.real" ]; then
    mv "$BUILD_DIR/usr/bin/dpkg-divert" "$BUILD_DIR/usr/bin/dpkg-divert.real"
    cat << 'EOF' > "$BUILD_DIR/usr/bin/dpkg-divert"
#!/bin/sh
for arg in "$@"; do
    if [ -d "$arg" ]; then
        exit 0
    fi
done
exec /usr/bin/dpkg-divert.real "$@"
EOF
    chmod +x "$BUILD_DIR/usr/bin/dpkg-divert"
fi

# Ensure dpkg architecture is explicitly recorded
mkdir -p "$BUILD_DIR/var/lib/dpkg" "$BUILD_DIR/etc/dpkg"
echo "amd64" > "$BUILD_DIR/var/lib/dpkg/arch"
echo "amd64" > "$BUILD_DIR/etc/dpkg/arch"

echo ">>> Step 5/7: Cleaning caches to keep initramfs compact..."
rm -rf "$BUILD_DIR/var/cache/apt/archives/"*.deb
rm -rf "$BUILD_DIR/usr/share/doc/"*
rm -rf "$BUILD_DIR/usr/share/man/"*
rm -rf "$BUILD_DIR/tmp/"*

echo ">>> Step 6/7: Swapping rootfs..."
BACKUP_DIR="$PROJECT_DIR/rootfs_backup_$(date +%s)"
echo "    Backing up existing rootfs to: $BACKUP_DIR"
mv "$ROOTFS" "$BACKUP_DIR"
mv "$BUILD_DIR" "$ROOTFS"

# Fix ownership
chown -R 1000:1000 "$ROOTFS/home/leviathan" 2>/dev/null || true

echo ">>> Step 7/7: Rebuilding Leviathan ISO..."
cd "$PROJECT_DIR"
make iso

echo ""
echo "========================================================="
echo "  SUCCESS! Official base installed and ISO rebuilt."
echo "  To launch: cd '$PROJECT_DIR' && ./run.sh"
echo "========================================================="
