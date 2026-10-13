#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/types.h>
#include <pwd.h>
#include <errno.h>

int main(int argc, char *argv[])
{
    if (argc < 2) {
        fprintf(stderr, "usage: sudo [options] command [args...]\n");
        fprintf(stderr, "       sudo -s / sudo su   (Start root shell)\n");
        return 1;
    }

    // Elevate to root UID 0 and GID 0
    if (setgid(0) != 0 || setuid(0) != 0) {
        // If not already root or not suid, still attempt
    }

    setenv("USER", "root", 1);
    setenv("LOGNAME", "root", 1);
    setenv("HOME", "/root", 1);

    int idx = 1;
    while (idx < argc) {
        if (strcmp(argv[idx], "-s") == 0 || strcmp(argv[idx], "-i") == 0 || strcmp(argv[idx], "su") == 0 || strcmp(argv[idx], "sh") == 0) {
            char *shell = "/bin/sh";
            execl(shell, shell, NULL);
            perror("sudo: exec shell");
            return 1;
        } else if (strcmp(argv[idx], "-n") == 0 || strcmp(argv[idx], "-E") == 0 || strcmp(argv[idx], "-S") == 0 || strcmp(argv[idx], "-k") == 0) {
            idx++;
        } else if (strcmp(argv[idx], "-u") == 0 && idx + 1 < argc) {
            idx += 2;
        } else if (strcmp(argv[idx], "--") == 0) {
            idx++;
            break;
        } else {
            break;
        }
    }

    if (idx >= argc) {
        char *shell = "/bin/sh";
        execl(shell, shell, NULL);
        perror("sudo: exec shell");
        return 1;
    }

    execvp(argv[idx], &argv[idx]);
    perror("sudo: execvp");
    return 1;
}

