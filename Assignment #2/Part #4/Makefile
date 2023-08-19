# HOW TO RUN THE CODE
# 	COMPILING THE MODULES
# 		make sure you are located in the directory with all of
# 		the proper files and run the following command three times
#
#			(1) run it to see if there are any symbols missing
#			(2) run with the commented section uncommented to generated an exports header file
#					make sure to remove the "init_module" and "cleanup_module" sections from the file
#			(3) run it again to compile completely and be ready to insert/remove
#
# 		make -C ~/linux_with_rust M=$PWD
#
#	INSERT/REMOVE THE MODULES
#		you can insert/remove modules using the
#		corresponding definitions in the Makefile

obj-m := rust_camera.o
rust_camera.o: rust_camera.rs

# rm -v !("rust_camera.rs"|"Makefile")

camera:
	clear; make -C ~/linux_with_rust M=$(PWD)

load:
	sudo insmod rust_camera.ko
	sudo chmod 766 /dev/RustCamera

unload:
	sudo rmmod rust_camera.ko

cl_dmesg:
	sudo dmesg -c
	clear
