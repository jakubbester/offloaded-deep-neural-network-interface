/*
 * Part #1 - HELLO WORLD KERNEL MODULE implementation.
 * @author Jakub Bester
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/init.h>

MODULE_AUTHOR("Jakub Bester");
MODULE_DESCRIPTION("Hello World Kernel Module");
MODULE_LICENSE("GPL");

/*
 * @brief runs when opening the module
 */
static int __init hello_init(void) {
	printk(KERN_INFO "Hello world!\n");
	return 0;
}

/*
 * @brief runs when closing the module
 */
static void __exit hello_cleanup(void) {
	printk(KERN_INFO "Goodbye world!");
}

module_init(hello_init);
module_exit(hello_cleanup);
