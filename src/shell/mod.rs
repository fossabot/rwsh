use crate::parser::parse_line;
use dirs;
use std::env;
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

#[derive(Default)]
pub struct Shell;

impl Shell {
    pub fn new() -> Shell {
        Shell
    }

    pub fn run(&self) {
        loop {
            print!("> ");
            stdout().flush().unwrap();
            let mut line = String::new();
            if stdin().read_line(&mut line).unwrap() == 0 {
                exit(0);
            }
            let mut _parts = parse_line(line.trim());
            let mut parts = _parts.iter().map(|x| &x[..]);
            let command = parts.next();
            if command.is_none() {
                continue;
            }
            let args = parts;
            if let Err(error) = self.run_command(command.unwrap(), args) {
                eprintln!("{}", error);
            }
        }
    }
    fn do_cd<'a, I>(&self, mut args: I) -> Result<(), String>
    where
        I: Iterator<Item = &'a str>,
    {
        if let Some(arg) = args.next() {
            let path = expand_home(Path::new(arg));
            match env::set_current_dir(path) {
                Err(error) => Err(format!("cd: {}", error)),
                _ => Ok(()),
            }
        } else {
            Err(String::from("cd needs an argument"))
        }
    }
    fn run_command<'a, I>(&self, command: &str, args: I) -> Result<(), String>
    where
        I: Iterator<Item = &'a str>,
    {
        match command {
            "cd" => self.do_cd(args),
            _ => match Command::new(command).args(args).spawn() {
                Ok(mut child) => {
                    if let Err(error) = child.wait() {
                        return Err(format!("rwsh: {}", error));
                    }
                    Ok(())
                }
                Err(error) => Err(format!("rwsh: {}", error)),
            },
        }
    }
}

fn expand_home<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut new_path = PathBuf::new();
    let mut it = path.as_ref().iter().peekable();

    if let Some(p) = it.peek() {
        if *p == "~" {
            new_path.push(dirs::home_dir().unwrap());
            it.next();
        }
    }
    for p in it {
        new_path.push(p);
    }
    new_path
}
