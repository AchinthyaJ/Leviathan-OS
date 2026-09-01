#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>

int main(int argc, char *argv[])
{
    if (argc != 3)
    {
        printf("Usage: cp SOURCE DEST\n");
        return 1;
    }

    int source = open(argv[1], O_RDONLY);

    if (source < 0)
    {
        printf("cp: cannot open source\n");
        return 1;
    }

    int destination = open(argv[2], O_WRONLY | O_CREAT | O_TRUNC, 0644);

    if (destination < 0)
    {
        printf("cp: cannot create destination\n");
        close(source);
        return 1;
    }

    char buffer[4096];
    ssize_t bytes_read;

    while ((bytes_read = read(source, buffer, sizeof(buffer))) > 0)
    {
        write(destination, buffer, bytes_read);
    }

    close(source);
    close(destination);

    return 0;
}

