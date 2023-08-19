/*
 * Part #2 - MEMORY DRIVER MODULE implementation using device files
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/init.h>
#include <linux/fs.h> // "filesystem" header allows us to create device files
#include <linux/slab.h> // included for the kmalloc operation
#include <linux/uaccess.h>
#include <linux/errno.h>

// MODULE GENERAL INFORMATION
MODULE_AUTHOR("Jakub Bester");
MODULE_DESCRIPTION("Simple hello world kernel module");
MODULE_LICENSE("GPL");

// DEFINE PROTOTYPES FOR FOPS STRUCT
int init_module(void);
void cleanup_module(void);
static int device_open(struct inode *, struct file *);
static ssize_t device_read(struct file *, char *, size_t, loff_t *);
static ssize_t device_write(struct file *, const char *, size_t, loff_t *);
static loff_t device_llseek(struct file *, loff_t, int);
static int device_close(struct inode *, struct file *);

#define SUCCESS 0
#define DEVICE_NAME "mymem" // name of device file
#define BUFFER_SIZE 524288 // max length of the buffer (512 KB)

static int major;
static int open = 0;
static char *msg_ptr;
static char *msg_start;
static char *msg_end;

// DEVICE FILE IMPLEMENTATION
static struct file_operations fops = {
    .owner = THIS_MODULE,
    .open = device_open,
    .read = device_read,
    .write = device_write,
    .llseek = device_llseek,
    .release = device_close
};

// MODULE IMPLEMENTATION
static int __init memory_init(void) {
    major = register_chrdev(0, DEVICE_NAME, &fops);

    if (major < 0) {
        printk("Registering the device failed %d", major);
        return major;
    }

    msg_ptr = (char *)kmalloc(sizeof(char) * BUFFER_SIZE, GFP_KERNEL);
    msg_start = msg_ptr;
    msg_end = msg_start + BUFFER_SIZE;
    

    printk("Device created at Major %d", major);
    printk(KERN_INFO "Creating memory module");
	return SUCCESS;
}

static void __exit memory_cleanup(void) {
    unregister_chrdev(major, DEVICE_NAME);
	printk(KERN_INFO "Cleaning up memory module");
}

// DEVICE FILE IMPLEMENTATION
static int device_open(struct inode *inode, struct file *file) {
    if (open) {
        return -EBUSY;
    }

    open++;

    return SUCCESS;
}

static ssize_t device_read(struct file *file, char *buffer, size_t length, loff_t *offset) {
    int bytes = 0;

    if (*msg_ptr == 0) {
        return SUCCESS; // signify that you have reached the end of the file
    }

    printk("read bytes");

    while (length && *msg_ptr) {
        put_user(*(msg_ptr++), buffer++);

        length--;
        bytes++;
    }

    return bytes;
}

static ssize_t device_write(struct file *file, const char *buffer, size_t length, loff_t *offset) {
    if (length > BUFFER_SIZE) {
        return -EINVAL;
    }

    printk("Device write works.");

    if (copy_from_user(msg_ptr + *offset, buffer, length)) {
        printk(KERN_ERR "Unable to read buffer from user.\n");
        return -EFAULT;
    }

    printk(KERN_INFO "Received %zu bytes from the user", length);
    return length;
}

static loff_t device_llseek(struct file *, loff_t offset, int flag) {
    // DETERMINE WHICH FLAG IS SET
    switch (flag) {
        case 0: // SEEK_SET
            if (msg_ptr + offset < msg_start) {
                printk(KERN_ERR "Offset would set past the beginning of the file");
                return -EINVAL;
            }

            msg_ptr = msg_start + offset;
            break;
        case 1: // SEEK_CUR
            if (msg_ptr + offset < msg_start) {
                printk(KERN_ERR "Offset would set past the beginning of the file");
                return -EINVAL;
            }

            msg_ptr += offset;
            break;
        case 2: // SEEK_END
            msg_ptr = msg_end + offset;
            break;
        default: // DEFAULT
            return -EINVAL;
            break;
    }

    return 0;
}

static int device_close(struct inode *inode, struct file *file) {
    open--;
    return SUCCESS;
}

module_init(memory_init);
module_exit(memory_cleanup);

/**
 * RUNNING THE CODE
 * NOW CONTAINED WITHIN THE MAKEFILE (make load) :
 *      sudo insmod NAME_OF_MODULE                        : INSERT/CREATE THE MODULE
 *      sudo mknod -m 666 /dev/DEVICE_NAME c MAJOR 0      : CREATES A DEVICE FILE
 * NOW CONTAINED WITHIN THE MAKEFILE (make unload) :
 *      sudo rm /dev/DEVICE_NAME                          : REMOVE THE DEVICE FILE
 *      sudo rmmod NAME_OF_MODULE                         : REMOVES THE MODULE
 * sudo dmesg | tail -n NUMBER_OF_LINES                   : DISPLAYS THE LOG WITH HOWEVER MANY LINES SPECIFIED
 * sudo dmesg                                             : PRINT OUT THE KERNEL LOG
 * ls -l /dev                                             : VIEW ALL DEVICE FILES
 */
