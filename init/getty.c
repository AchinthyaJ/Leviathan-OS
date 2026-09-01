#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <errno.h>

int main(void)
{
    int tty;

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
    tty = open("/dev/tty1", O_RDWR);

    if (tty < 0)
    {
        perror("getty: open /dev/tty1");
        return 1;
    }

    /*
     * Make tty1 our controlling terminal.
     */
    if (ioctl(tty, TIOCSCTTY, 0) < 0)
    {
        perror("getty: TIOCSCTTY");
        close(tty);
        return 1;
    }

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

