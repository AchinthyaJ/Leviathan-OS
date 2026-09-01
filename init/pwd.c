#include <stdio.h>
#include <unistd.h>

int main(int argc, char *argv[])
{
    (void)argv;

    if (argc > 1)
    {
        fprintf(stderr, "Usage: pwd\n");
        return 1;
    }

    char cwd[4096];

    if (getcwd(cwd, sizeof(cwd)) == NULL)
    {
        perror("pwd");
        return 1;
    }

    printf("%s\n", cwd);

    return 0;
}
