#![no_std]
#![no_main]

use users::syscall::sleep;

#[macro_use]
extern crate users;
// extern crate alloc;

#[no_mangle]
fn main() {
    println!("Hello World!");
    sleep(5000);
    println!("Sleep End!");
}
