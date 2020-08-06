use std::io::{self, BufRead, BufReader, Write};
use std::fs::{self, File, OpenOptions};
use std::process::Stdio;
use os_pipe::{dup_stderr, dup_stdin, dup_stdout, PipeReader, PipeWriter};
use std::collections::HashMap;

// My own, less nasty version of BufRead::lines().
// Returns an Option rather Option<Result>,
// and keeps the newline
#[derive(Debug)]
pub struct Lines<B> {
    buf: B,
}

impl<B> Lines<B> {
    pub fn new(buf: B) -> Lines<B> {
        Lines { buf }
    }
}

impl<B: BufRead> Iterator for Lines<B> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let mut buf = String::new();
        match self.buf.read_line(&mut buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(buf),
        }
    }
}

pub struct Shell {
    lines: Lines<Box<dyn BufRead>>,
    interactive: bool,
    pub vars: HashMap<String, String>,
}

impl Shell {
    pub fn new(file: Option<String>) -> Shell {
        let (lines, interactive): (Lines<Box<dyn BufRead>>, bool) = if let Some(filename) = file {
            (Lines::new(Box::new(BufReader::new(fs::File::open(filename).unwrap()))), false)
        } else {
            (Lines::new(Box::new(BufReader::new(io::stdin()))), true)
        };
        Shell {
            lines, 
            interactive,
            vars: HashMap::new(),
        }
    }

    pub fn is_interactive(&self) -> bool {
        self.interactive
    }

    pub fn next_prompt(&mut self, prompt: &str) -> Option<String> {
        if self.interactive {
            print!("{}", prompt);
            io::stdout().flush().unwrap();
        }
        self.lines.next()
    }
}

impl Iterator for Shell {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.interactive {
            print!("~> ");
            io::stdout().flush().unwrap();
        }
        self.lines.next()
    }

}

// File descriptor - somewhat a misnomer now but it's nice and short.
// Keeps track of the various ports a stdio could be connected to.
#[derive(Debug)]
pub enum Fd {
    Stdin,
    Stdout,
    Stderr,
    Inherit,
    PipeOut(PipeWriter),
    PipeIn(PipeReader),
    FileName(String),
    FileNameAppend(String),
    RawFile(File),
}

impl PartialEq for Fd {
    fn eq(&self, other: &Self) -> bool {
        self.variant() == other.variant()
    }
}

impl Fd {
    fn variant(&self) -> &str {
        match *self {
            Fd::Stdin => "Stdin",
            Fd::Stdout => "Stdout",
            Fd::Stderr => "Stderr",
            Fd::Inherit => "Inherit",
            Fd::PipeOut(_) => "PipeOut",
            Fd::PipeIn(_) => "PipeIn",
            Fd::FileName(_) => "FileName",
            Fd::FileNameAppend(_) => "FileNameAppend",
            Fd::RawFile(_) => "RawFile", // Not completely accurate, but I think fine for now
        }
    }

    // Gets an stdin - all same here as stdout, except that a file is opened, not created
    pub fn get_stdin(&mut self) -> Option<Stdio> {
        match self {
            Fd::FileName(name) => match File::open(&name) {
                Ok(file) => {
                    *self = Fd::RawFile(file.try_clone().unwrap());
                    Some(Stdio::from(file))
                }
                Err(e) => {
                    eprintln!("rush: {}: {}", name, e);
                    None
                }
            },
            _ => self.get_stdout(),
        }
    }

    // All the ways a Fd could be converted to a Stdio
    // What's the proper way to deal with all of these dup unwraps?
    // What is their fail condition?
    pub fn get_stdout(&mut self) -> Option<Stdio> {
        match self {
            Fd::Stdin => Some(Stdio::from(dup_stdin().unwrap())),
            Fd::Stdout => Some(Stdio::from(dup_stdout().unwrap())),
            Fd::Stderr => Some(Stdio::from(dup_stderr().unwrap())),
            Fd::Inherit => Some(Stdio::inherit()),
            Fd::PipeOut(writer) => Some(Stdio::from(writer.try_clone().unwrap())),
            Fd::PipeIn(reader) => Some(Stdio::from(reader.try_clone().unwrap())),
            Fd::RawFile(file) => Some(Stdio::from(file.try_clone().unwrap())),
            Fd::FileName(name) => match File::create(&name) {
                Ok(file) => {
                    *self = Fd::RawFile(file.try_clone().unwrap());
                    Some(Stdio::from(file))
                }
                Err(e) => {
                    eprintln!("rush: {}: {}", name, e);
                    None
                }
            },
            Fd::FileNameAppend(name) => {
                match OpenOptions::new().append(true).create(true).open(&name) {
                    Ok(file) => {
                        *self = Fd::RawFile(file.try_clone().unwrap());
                        Some(Stdio::from(file))
                    }
                    Err(e) => {
                        eprintln!("rush: {}: {}", name, e);
                        None
                    }
                }
            }
        }
    }

    pub fn get_stderr(&mut self) -> Option<Stdio> {
        self.get_stdout()
    }
}
