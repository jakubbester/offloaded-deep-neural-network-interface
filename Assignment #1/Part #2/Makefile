
obj-m += memory.o

all:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) modules

clean:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) clean

test: test.c
	gcc test.c -o test

load:
	sudo insmod memory.ko
	sudo mknod -m 666 /dev/mymem c 235 0 

unload:
	sudo rm /dev/mymem
	sudo rmmod memory
