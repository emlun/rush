use crate::lexer::Lexer;
use crate::parser::Cmd;
use crate::parser::Parser;
use regex::Regex;
use std::collections::BTreeMap;
use std::process::exit as exit_program;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use crate::helpers::Shell;

// Unless specified otherwise, if provided multiple arguments while only
// accepting one, these use the first argument. Dash does this as well.  

fn escape_singlequotes(s: &str) -> String {
    s.replace("'", r"\'")
}

pub fn alias(shell: &Rc<RefCell<Shell>>, args: Vec<String>) -> bool {
    if args.is_empty() {
        for (lhs, rhs) in &shell.borrow_mut().aliases {
            println!(
                "alias {}='{}'",
                lhs,
                rhs.as_ref()
                    .map(|cmd| cmd.to_commandline())
                    .map(|cmd| escape_singlequotes(&cmd))
                    .unwrap_or("".to_string())
            );
        }
        true
    } else {
        let mut success = true;
        let assignment_re = Regex::new(r"^(\w+)=(.*)").unwrap();
        for arg in args {
            if assignment_re.is_match(&arg) {
                let caps = assignment_re.captures(&arg).unwrap();
                let lhs = &caps[1];
                let rhs = &caps[2];

                let lexer = Lexer::new(rhs, Rc::clone(shell));
                let mut parser = Parser::new(lexer, Rc::clone(shell));

                if let Ok(substitution) = parser.get() {
                    shell
                        .borrow_mut()
                        .aliases
                        .insert(lhs.to_string(), Some(substitution));
                } else {
                    shell.borrow_mut().aliases.insert(lhs.to_string(), None);
                }
            } else if shell.borrow().aliases.contains_key(&arg) {
                println!(
                    "alias {}='{}'",
                    arg,
                    shell.borrow().aliases[&arg]
                        .as_ref()
                        .map(|cmd| cmd.to_commandline())
                        .map(|cmd| escape_singlequotes(&cmd))
                        .unwrap_or("".to_string())
                );
            } else {
                eprintln!("rush: alias: {}: not found", arg);
                success = false;
            }
        }
        success
    }
}

pub fn exit(args: Vec<String>) -> bool {
    match args.get(0).map_or(Ok(0), |x| x.parse::<i32>()) {
        Ok(n) => {
            exit_program(n);
        },
        Err(e) => {
            eprintln!("rush: {}", e);
            false
        },
    }
}

pub fn cd(args: Vec<String>) -> bool {
    let new_dir = args.into_iter().next().unwrap_or_else(|| env::var("HOME").unwrap());
    if let Err(e) = env::set_current_dir(new_dir) {
        eprintln!("rush: {}", e);
        false
    } else {
        true
    }
}

// Set very versetaile normally, this is just positional parameters for now
pub fn set(args: Vec<String>, shell: &Rc<RefCell<Shell>>) -> bool {
    shell.borrow_mut().set_pos(args);
    true
}

pub fn unalias(aliases: &mut BTreeMap<String, Option<Cmd>>, args: Vec<String>) -> bool {
    if args.is_empty() {
        eprintln!("unalias: usage: unalias [-a] name [name ...]");
        false
    } else if args[0] == "-a" {
        aliases.clear();
        true
    } else {
        let mut success = true;
        for arg in args {
            if aliases.contains_key(&arg) {
                aliases.remove(&arg);
            } else {
                eprintln!("rush: unalias: {}: not found", arg);
                success = false;
            }
        }
        success
    }
}
