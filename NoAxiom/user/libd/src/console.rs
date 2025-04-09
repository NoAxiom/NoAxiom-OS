#![allow(unused)]

use alloc::string::String;
extern crate core;
use core::fmt::{self, Write};

use crate::syscall::{read, write};

const STDIN: usize = 0;
const STDOUT: usize = 1;

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(STDOUT, s.as_bytes());
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

pub fn get_char() -> u8 {
    let mut c = [0u8, 1];

    loop {
        read(STDIN, &mut c);
        if c[0] == 127 {
            continue;
        }
        if c[0] < 32 {
            continue;
        }
        break;
    }
    c[0]
}

pub fn read_line() -> String {
    let mut c = [0u8, 1];
    let mut res = String::new();
    loop {
        read(STDIN, &mut c);
        if c[0] == b'\n' || c[0] == b'\r' {
            break;
        }
        if c[0] == 127 {
            if res.len() > 0 {
                print!("\x08 \x08");
                res.pop();
            }
            continue;
        }
        if c[0] < 32 {
            continue;
        }
        print!("{}", c[0] as char);
        res.push(c[0] as char);
    }
    println!("");
    res
}

#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[91m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}

#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[92m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}

#[macro_export]
macro_rules! warn {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[93m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}
#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!("\x1b[94m{}\x1b[0m",format_args!(concat!($fmt, "\n") $(, $($arg)+)?)));
    }
}
