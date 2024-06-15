#![allow(unused_macros)]
#![allow(unused_imports)]

use std::io::Write;

pub fn process_msg(msg: &str) -> String {
    let mut processed_msg: String = String::new();

    let mut bold_count: usize = 0;
    let mut highlight_count: usize = 0;

    for (i, char) in msg.char_indices() {
        if char == '*' && msg.chars().nth(i.saturating_sub(1)) != Some('\\') {
            processed_msg.push_str(if bold_count % 2 == 0 {
                "\x1b[1m"
            } else {
                "\x1b[0m"
            });
            bold_count += 1;
        } else if char == '~' && msg.chars().nth(i.saturating_sub(1)) != Some('\\') {
            processed_msg.push_str(if highlight_count % 2 == 0 {
                "\x1b[1;36m"
            } else {
                "\x1b[0m"
            });
            highlight_count += 1;
        } else if char == '\\' && [Some('*'), Some('~')].contains(&msg.chars().nth(i + 1)) {
            continue;
        } else {
            processed_msg.push(char);
        }
    }

    processed_msg
}

pub fn _info<S: AsRef<str>>(msg: S, end: S) {
    let mut stdout: std::io::Stdout = std::io::stdout();

    stdout
        .write((String::from(" :: ") + &process_msg(msg.as_ref()) + end.as_ref()).as_bytes())
        .ok();

    stdout.flush().ok();
}

pub fn _warn<S: AsRef<str>>(msg: S, end: S) {
    let mut stderr: std::io::Stderr = std::io::stderr();

    stderr
        .write(
            (String::from(" :: \x1b[1;33mwarning\x1b[0m: ")
                + &process_msg(msg.as_ref())
                + end.as_ref())
            .as_bytes(),
        )
        .ok();

    stderr.flush().ok();
}

pub fn _error<S: AsRef<str>>(msg: S, end: S) {
    let mut stderr: std::io::Stderr = std::io::stderr();

    stderr
        .write(
            (String::from(" :: \x1b[1;31merror\x1b[0m: ")
                + &process_msg(msg.as_ref())
                + end.as_ref())
            .as_bytes(),
        )
        .ok();

    stderr.flush().ok();
}

macro_rules! infoln {
    ($($arg:tt)*) => {
        recli::_info(&*std::format!($($arg)*), "\n")
    };
}

macro_rules! info {
    ($($arg:tt)*) => {
        recli::_info(&*std::format!($($arg)*), "")
    };
}

macro_rules! warnln {
    ($($arg:tt)*) => {
        recli::_warn(&*std::format!($($arg)*), "\n")
    };
}

macro_rules! errnln {
    ($($arg:tt)*) => {
        recli::_error(&*std::format!($($arg)*), "\n")
    };
}

macro_rules! panicln {
    ($($arg:tt)*) => {{
        recli::_error(&*std::format!($($arg)*), "\n");
        std::process::exit(1)
    }};
}

macro_rules! clreset {
    () => {
        use std::io::Write;
        write!(std::io::stdout(), "\x1b[0m").ok();
        write!(std::io::stderr(), "\x1b[0m").ok();
    };
}

pub(crate) use clreset;
pub(crate) use errnln;
pub(crate) use info;
pub(crate) use infoln;
pub(crate) use panicln;
pub(crate) use warnln;
