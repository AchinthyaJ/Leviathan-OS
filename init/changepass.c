/*
 * changepass.c — Leviathan OS password changer
 *
 * Usage (as root inside the OS):
 *   changepass "newpassword"
 *
 * Updates the SHA-512 crypt hash for the 'leviathan' user in /etc/shadow.
 * Generates a random 16-character salt for each invocation.
 *
 * Must run as uid 0 (root).
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>
#include <crypt.h>
#include <errno.h>

#define SHADOW_PATH   "/etc/shadow"
#define SHADOW_TMP    "/etc/shadow.tmp"
#define TARGET_USER   "leviathan"
#define SALT_CHARS    "abcdefghijklmnopqrstuvwxyz" \
                      "ABCDEFGHIJKLMNOPQRSTUVWXYZ" \
                      "0123456789./"
#define SALT_LEN      16

/* Generate a random SHA-512 crypt salt: $6$<16 chars>$ */
static void make_salt(char *out, size_t outsz) {
    static const char chars[] = SALT_CHARS;
    int fd;
    unsigned char rnd[SALT_LEN];

    fd = open("/dev/urandom", O_RDONLY);
    if (fd < 0) {
        /* Fallback to time-seeded rand */
        srand((unsigned)time(NULL) ^ (unsigned)getpid());
        for (int i = 0; i < SALT_LEN; i++)
            rnd[i] = (unsigned char)rand();
    } else {
        read(fd, rnd, sizeof(rnd));
        close(fd);
    }

    snprintf(out, outsz, "$6$");
    for (int i = 0; i < SALT_LEN; i++)
        out[3 + i] = chars[rnd[i] % (sizeof(chars) - 1)];
    out[3 + SALT_LEN] = '$';
    out[3 + SALT_LEN + 1] = '\0';
}

int main(int argc, char *argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Usage: changepass <newpassword>\n");
        return 1;
    }
    if (geteuid() != 0) {
        fprintf(stderr, "changepass: must run as root\n");
        return 1;
    }

    const char *newpass = argv[1];

    /* Generate hash */
    char salt[32];
    make_salt(salt, sizeof(salt));

    struct crypt_data cdata;
    memset(&cdata, 0, sizeof(cdata));
    const char *hashed = crypt_r(newpass, salt, &cdata);
    if (!hashed) {
        perror("crypt_r");
        return 1;
    }

    /* Read shadow, update leviathan's entry */
    FILE *fin = fopen(SHADOW_PATH, "r");
    if (!fin) {
        perror("open " SHADOW_PATH);
        return 1;
    }
    FILE *fout = fopen(SHADOW_TMP, "w");
    if (!fout) {
        perror("open " SHADOW_TMP);
        fclose(fin);
        return 1;
    }

    char line[4096];
    int updated = 0;

    while (fgets(line, sizeof(line), fin)) {
        /* Check if this is the target user line */
        size_t ulen = strlen(TARGET_USER);
        if (strncmp(line, TARGET_USER ":", ulen + 1) == 0) {
            /* Parse: username:hash:rest */
            char *p = line + ulen + 1;          /* skip "leviathan:" */
            char *after_hash = strchr(p, ':');  /* find second ':' */
            if (after_hash) {
                /* Write updated line */
                fprintf(fout, "%s:%s%s", TARGET_USER, hashed, after_hash);
                updated = 1;
                continue;
            }
        }
        fputs(line, fout);
    }

    fclose(fin);
    fclose(fout);

    if (!updated) {
        fprintf(stderr, "changepass: user '%s' not found in %s\n",
                TARGET_USER, SHADOW_PATH);
        unlink(SHADOW_TMP);
        return 1;
    }

    /* Atomically replace */
    if (rename(SHADOW_TMP, SHADOW_PATH) != 0) {
        perror("rename");
        unlink(SHADOW_TMP);
        return 1;
    }

    /* Lock down permissions */
    chmod(SHADOW_PATH, 0640);

    printf("Password for '%s' updated successfully.\n", TARGET_USER);
    return 0;
}
