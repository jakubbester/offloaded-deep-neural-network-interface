#include <stdio.h>
#include <pthread.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdlib.h>
#include <errno.h>

#define INIT_VAL "DEADBEEF" // to be turned into a 64-bit unsigned integer
#define EIGHT_BYTES 8
#define WORKERS 50 // this is the # of (W)orker threads to be created
#define NUMBER 200 // the amount of work to be performed by each worker thread
#define INIT_FILE_LOC 0

// static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER; // MUTUAL EXCLUSION : a lock that can be set or unset
pthread_mutex_t mutex;

int file_descriptor; // how to access file after opening /dev/mymem

void *worker_thread(void *arguments) {
    // PERFORM THE FOLLOWING WORKLOAD (N)UMBER TIMES
    //      read the counter value from the first eight bytes
    //      increment the counter value by one
    //      write the newly-incremented counter value back to the memory region
    
    for (size_t i = 0; i < NUMBER; i++) {
        // ACQUIRE A LOCK ON THE DEVICE FILE
        int val2 = pthread_mutex_lock(&mutex);
        if (val2 != 0) {
            printf("NOT WORKING");
        }

        long long int val; // declare storage variable

        // READ THE VALUE STORED IN DEVICE FILE AND PRINT IT
        lseek(file_descriptor, INIT_FILE_LOC, SEEK_SET);
        read(file_descriptor, &val, EIGHT_BYTES);
        
        val += 1; // increment by one

        // WRITE THE VALUE BACK INTO THE DEVICE FILE
        lseek(file_descriptor, INIT_FILE_LOC, SEEK_SET);
        write(file_descriptor, &val, 600000);

        // RELEASE THE LOCK ON THE DEVICE FILE
        pthread_mutex_unlock(&mutex);
    }
    
    pthread_exit(NULL);
}

int main() {
    // OPEN DEVICE FILE TO READ AND WRITE TO
    file_descriptor = open("/dev/mymem", O_RDWR);
    if (file_descriptor < 0) {
        perror("Not able to open device");
        return errno;
    }

    // WRITE EIGHT BYTES TO THE FILE
    lseek(file_descriptor, INIT_FILE_LOC, SEEK_SET);
    write(file_descriptor, INIT_VAL, EIGHT_BYTES);
    
    // CREATE THE REQUIRED THREADS IN A LOOP
    for (size_t i = 0; i < WORKERS; i++) {
        pthread_t thd; // create the thread
        pthread_create(&thd, NULL, worker_thread, NULL);

        pthread_join(thd, NULL); // wait for all of the threads to finish
    }

    // PRINT THE FINAL RESULT
    long long int val;
    lseek(file_descriptor, INIT_FILE_LOC, SEEK_SET);
    read(file_descriptor, &val, 8);
    printf("%lld\n", val);

    close(file_descriptor);
    return EXIT_SUCCESS;
}
