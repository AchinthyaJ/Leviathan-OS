#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <errno.h>

int main(int argc, char *argv[])
{
    int tty;
    const char *dev = (argc > 1) ? argv[1] : "/dev/tty1";

    /*
     * Become a session leader.
     */
    if (setsid() < 0)
    {
        perror("getty: setsid");
        return 1;
    }

    /*
     * Open Leviathan's terminal.
     */
    tty = open(dev, O_RDWR);
    if (tty < 0)
    {
        tty = open("/dev/console", O_RDWR);
    }

    if (tty < 0)
    {
        perror("getty: open terminal");
        return 1;
    }

    /*
     * Reactivate VT text mode, unhide cursor, and clear screen
     */
    ioctl(tty, 0x4B3A /* KDSETMODE */, 0 /* KD_TEXT */);
    ioctl(tty, 0x5606 /* VT_ACTIVATE */, 1);
    const char *reset_seq = "\033[?25h\033[2J\033[H";
    if (write(tty, reset_seq, 13) < 0) {
        // Non-fatal if terminal reset write fails
    }

    /*
     * Make terminal our controlling terminal (force steal if needed).
     */
    ioctl(tty, TIOCSCTTY, 1);

    /*
     * Connect stdin, stdout and stderr to the TTY.
     */
    if (dup2(tty, STDIN_FILENO) < 0 ||
        dup2(tty, STDOUT_FILENO) < 0 ||
        dup2(tty, STDERR_FILENO) < 0)
    {
        perror("getty: dup2");
        close(tty);
        return 1;
    }

    if (tty > STDERR_FILENO)
        close(tty);

    printf("\n");
    printf("================================\n");
    printf("        LEVIATHAN TTY\n");
    printf("================================\n");
    printf("\n");

    /*
     * For now, BusyBox is our temporary shell.
     * Later this becomes /bin/leviathan-sh.
     */
    execl("/bin/sh", "sh", NULL);

    perror("getty: exec /bin/sh");

    return 1;
}

