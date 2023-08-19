
obj-m += memory.o

# BUILDING THE MODULE

all:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) modules

clean:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) clean

# TEST.RS SECTION

test:
	rustc test.rs

tclean:
	rm test results

# THREAD.RS SECTION

thread:
	rustc thread.rs

thclean:
	rm thread

# LOADING AND REMOVING THE C MODULE

loadc:
	sudo insmod memory.ko
	sudo mknod -m 666 /dev/mymem c 235 0

unloadc:
	sudo rm /dev/mymem
	sudo rmmod memory

# LOADING AND REMOVING THE RUST MODULE
#	This would have to be compiled along with
# 	all of the other modules accordingly before
# 	being inserted in.

loadr:
	sudo insmod memory.ko
	sudo chmod 766 /dev/mymem

unloadr:
	sudo rm /dev/mymem
