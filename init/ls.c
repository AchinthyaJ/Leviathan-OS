#include <stdio.h>
#include <dirent.h>

int main(int argc, char *argv[])
{
    if (argc > 2)
    {
        fprintf(stderr, "Usage: ls [DIRECTORY]\n");
        return 1;
    }

    const char *path = (argc == 2) ? argv[1] : ".";

    DIR *dir = opendir(path);
    if (!dir)
    {
        perror("ls");
        return 1;
    }

    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL)
    {
        printf("%s\n", entry->d_name);
    }

    closedir(dir);
    return 0;
}
