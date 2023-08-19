# SPDX-License-Identifier: GPL-2.0

obj-m += rust_camera.o
rust_camera.o: rust_camera.rs

camera:
	clear; make -C ~/linux_with_rust M=$(PWD)

insmod:
	sudo insmod rust_camera_wrapper.ko
	sudo dmesg

rmmod:
	sudo rmmod rust_camera_wrapper
	sudo dmesg
