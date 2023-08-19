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
# 		KRUSTFLAGS="--emit=metadata --extern mymem=
#			/home/jakub/Documents/Assignment1/Part5/librust_mymem.rmeta" make -C ~/linux_with_rust M=$PWD
#
#	INSERT/REMOVE THE MODULES
#		you can insert/remove modules using the
#		corresponding definitions in the Makefile

obj-m := final_rust_mymem.o rust_mymem_thread.o
final_rust_mymem-y := rust_mymem.o mymem_exports.o 

quiet_cmd_exports = EXPORTS $@
    cmd_exports = \
		$(NM) -p --defined-only $< \
		| grep -E ' (T|R|D) ' | cut -d ' ' -f 3 \
		| xargs -Isymbol \
		echo 'EXPORT_SYMBOL_RUST_GPL(symbol);' > $@

rust_mymem.o: rust_mymem.rs

# $(obj)/exports_mymem_generated.h: $(obj)/rust_mymem.o FORCE
# 	$(call if_changed,exports)

load:
	sudo insmod final_rust_mymem.ko
	sudo insmod rust_mymem_thread.ko

unload:
	sudo rmmod rust_mymem_thread
	sudo rmmod final_rust_mymem
