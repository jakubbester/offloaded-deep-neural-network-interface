#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>

#define NUMBER_OF_TESTS 4
#define NUMBER_OF_TRIALS 10

int main() {
    // OPEN DEVICE FILE TO WRITE/READ TO/FROM
    int df = open("/dev/mymem", O_RDWR);
    if (df < 0) {
        perror("Not able to open device");
        return errno;
    }

    // CREATE RESULTS FILE TO PRINT TO
    FILE *fp = fopen("results", "w");

    // TEST WRITING 1 BYTE
    fprintf(fp, "1B TEST WRITE |");
    for (size_t i = 0; i < NUMBER_OF_TRIALS; i++) {
        lseek(df, 0, SEEK_SET);

        clock_t begin = clock();
        write(df, "A", strlen("A"));
        clock_t end = clock();

        fprintf(fp, " %lf |", (double)(end - begin) / CLOCKS_PER_SEC);
    }
    fprintf(fp, "\n");

    // TEST READING 1 BYTE
    fprintf(fp, "1B TEST READ  |");
    for (size_t i = 0; i < NUMBER_OF_TRIALS; i++) {
        lseek(df, 0, SEEK_SET);
        char tmp[1];

        clock_t begin = clock();
        read(df, tmp, strlen("A"));
        clock_t end = clock();

        fprintf(fp, " %lf |", (double)(end - begin) / CLOCKS_PER_SEC);
    }
    fprintf(fp, "\n");

    // PERFORM TESTS READING/WRITING 64 BYTES, 1024 BYTES, 65,536 BYTES, and 524,288 BYTES
    size_t sizes[4] = {64, 1024, 65536, 524288};
    char characters[4] = {'B', 'C', 'D', 'E'};
    for (size_t i = 0; i < NUMBER_OF_TESTS; i++) {
        // SET UP THE RESULTS DOCUMENT TO PRINT FINAL TIMES
        switch (i) {
            case 0:
                fprintf(fp, "64B TEST      |");
                break;
            case 1:
                fprintf(fp, "1kB TEST      |");
                break;
            case 2:
                fprintf(fp, "64kB TEST     |");
                break;
            case 3:
                fprintf(fp, "512kB TEST    |");
                break;
        }

        // SET UP CHARACTER ARRAYS TO READ AND WRITE TO
        char *tmp = (char *)malloc(sizeof(char) * sizes[i] + 1);
        for (size_t j = 0; j < sizes[i]; j++) {
            tmp[j] = characters[i];
        }
        tmp[sizes[i]] = '\0';

        // RUN THE TESTS AND MEASURE TIMES REQUIRED TO RUN
        for (size_t j = 0; j < NUMBER_OF_TRIALS; j++) {
            clock_t begin = clock();
            write(df, tmp, sizes[i]);
            read(df, tmp, sizes[i]);
            lseek(df, 0, SEEK_SET);
            clock_t end = clock();
 
            fprintf(fp, " %lf |", (double)(end - begin) / CLOCKS_PER_SEC);
        }

        fprintf(fp, "\n");
        free(tmp);
    }

    close(df);
    fclose(fp);
    return 0;
}
