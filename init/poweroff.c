/*
 * poweroff.c — Leviathan OS system power controller
 *
 * Handles poweroff, reboot, shutdown, and suspend without requiring systemd or PAM.
 * Statically compiled, zero runtime dependencies.
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/reboot.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <signal.h>

static void flush_and_sync(void) {
    printf("Leviathan: Syncing filesystems...\n");
    fflush(stdout);
    sync();
}

int main(int argc, char **argv) {
    const char *prog = argv[0];
    int do_reboot = 0;
    int do_suspend = 0;

    if (strstr(prog, "reboot")) {
        do_reboot = 1;
    } else if (strstr(prog, "suspend")) {
        do_suspend = 1;
    }

    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "-r") || !strcmp(argv[i], "--reboot") || !strcmp(argv[i], "reboot")) {
            do_reboot = 1;
        } else if (!strcmp(argv[i], "--suspend") || !strcmp(argv[i], "suspend")) {
            do_suspend = 1;
        }
    }

    if (do_suspend) {
        printf("Leviathan: Entering system suspend...\n");
        int fd = open("/sys/power/state", O_WRONLY);
        if (fd >= 0) {
            write(fd, "mem\n", 4);
            close(fd);
            return 0;
        }
        perror("open /sys/power/state");
        return 1;
    }

    flush_and_sync();

    // 1. Direct SysRq trigger for immediate hardware shutdown/reboot
    int sfd = open("/proc/sysrq-trigger", O_WRONLY);
    if (sfd >= 0) {
        if (do_reboot) {
            write(sfd, "b\n", 2);
        } else {
            write(sfd, "o\n", 2);
        }
        close(sfd);
    }

    // 2. Direct Linux reboot syscall
    if (do_reboot) {
        printf("Leviathan: Rebooting system now.\n");
        fflush(stdout);
        reboot(RB_AUTOBOOT);
    } else {
        printf("Leviathan: Powering off system now.\n");
        fflush(stdout);
        reboot(RB_POWER_OFF);
    }

    // 3. Fallback: ensure process does not return to callers
    while (1) {
        pause();
    }

    return 0;
}
