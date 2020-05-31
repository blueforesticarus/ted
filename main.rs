//tutorial-read-01.rs
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_macros)]
#![allow(unused_imports)]

use std::io::{stdin, stdout, Stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;

fn run() -> Result<Vec<csv::StringRecord>, Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut vec = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_reader(file);
    for result in rdr.records() {
        let record = result?;
        vec.push(record)
    }
    Ok(vec)
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

enum ViewMode {
    Column,
    Row,
}

enum InputMode {
    Normal,
    Insert,
    Command,
    Select,
}

enum Dir {
    Up,
    Down,
    Left,
    Right,
}

struct Mode {
    view: ViewMode,
    input: InputMode,
    position: (u16, u16), //row, column
    offset: u16,
    minimap: bool,
    minimap_width: u16,
    minimap_height: u16,
    dims: (u16, u16), //cols rows
    margin: u16,
    paste: String,
    command: String,
    stdout: termion::screen::AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    stale: bool,
    cursor: (u16, u16),
    exit: bool,
    no_clear: bool,
}

impl Mode {
    fn other_key(&mut self, c: char) {
        let code: String = c.escape_default().collect();
        write!(
            self.stdout,
            "{}{}{}",
            termion::cursor::Goto(self.dims.0 - code.len() as u16, self.dims.1),
            termion::clear::CurrentLine,
            code
        )
        .unwrap();
        self.no_clear = true;
    }
    fn move_cell(&mut self, dir: Dir) {
        match dir {
            Dir::Left => self.position.1 += 1,
            Dir::Right => {
                if self.position.1 > 0 {
                    self.position.1 -= 1;
                }
            }
            Dir::Up => {
                if self.position.0 > 0 {
                    self.position.0 -= 1;
                }
            }
            Dir::Down => self.position.0 += 1,
        }
    }
    fn enter_command(&mut self) {
        self.input = InputMode::Command;
        self.command.clear();
        self.cursor = (0, 0);
    }
    fn edit_cell(&self) {}
    fn change_cell(&self) {}
    fn delete_cell(&self) {}
    fn yank_cell(&self) {}
    fn paste_cell(&self) {}
    fn view_toggle(&self) {}
    fn minimap_toggle(&self) {}
    fn command_key(&mut self, c: char) {
        self.command.insert(self.cursor.1 as usize, c);
        self.cursor.1 += 1
    }
    fn command_del(&mut self) {
        if self.cursor.1 > 0 {
            self.command.remove(self.cursor.1 as usize - 1);
            self.cursor.1 -= 1;
        }
    }
    fn command_enter(&mut self) {
        self.input = InputMode::Normal;
        command_parse(self.command.clone().as_str(), self)
    }
    fn command_esc(&mut self) {
        self.input = InputMode::Normal;
    }
    fn insert_key(&self, c: char) {}
    fn insert_enter(&self) {}
    fn insert_esc(&self) {}
    fn insert_del(&self) {}
    fn move_insert(&self, dir: Dir) {}
}

fn command_parse(cmd: &str, mode: &mut Mode) {
    match cmd {
        "q" => mode.exit = true,
        _ => (),
    }
}

//PROBLEM: terminal cannot get keyup requests
//this means no chorded keybinds
fn main() {
    //setup input output
    //let ttyin = get_tty().unwrap();
    let mut cells = run().unwrap();

    let ttyin = stdin();
    let ttyout = termion::screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());
    let mut mode = Mode {
        view: ViewMode::Column,
        input: InputMode::Normal,
        position: (0, 0),
        offset: 0,
        minimap: true,
        minimap_width: 10,
        minimap_height: 6,
        dims: termion::terminal_size().unwrap(),
        margin: 11,
        paste: String::from(""),
        command: String::from(""),
        stdout: ttyout,
        stale: true,
        cursor: (0, 0),
        exit: false,
        no_clear: false,
    };

    //clear screen
    write!(
        mode.stdout,
        "{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1)
    )
    .unwrap();
    mode.stdout.flush().unwrap();

    //detecting keydown events
    draw(&mut mode, &cells);
    mode.stdout.flush().unwrap();
    for c in ttyin.keys() {
        mode.no_clear = false;
        key_delegate(c.unwrap(), &mut mode); // Print the key we type...
        draw(&mut mode, &cells);
        mode.stdout.flush().unwrap();
        if mode.exit {
            break;
        }
    }
}

fn key_delegate(key: Key, mode: &mut Mode) {
    match mode.input {
        InputMode::Normal => normal_mode(key, mode),
        InputMode::Command => command_mode(key, mode),
        InputMode::Insert => insert_mode(key, mode),
        InputMode::Select => (),
    }
}

fn normal_mode(key: Key, mode: &mut Mode) {
    match key {
        Key::Char('k') => mode.move_cell(Dir::Up),
        Key::Char('j') => mode.move_cell(Dir::Down),
        Key::Char('l') => mode.move_cell(Dir::Left),
        Key::Char('h') => mode.move_cell(Dir::Right),

        Key::Char(':') => mode.enter_command(),
        Key::Char(';') => mode.enter_command(),

        Key::Char('i') => mode.edit_cell(),
        Key::Char('c') => mode.change_cell(),
        Key::Char('d') => mode.delete_cell(),
        Key::Char('y') => mode.yank_cell(),
        Key::Char('p') => mode.paste_cell(),

        Key::Char('t') => mode.view_toggle(),
        Key::Char('m') => mode.minimap_toggle(),

        Key::Left => mode.move_cell(Dir::Up),
        Key::Right => mode.move_cell(Dir::Down),
        Key::Up => mode.move_cell(Dir::Left),
        Key::Down => mode.move_cell(Dir::Right),
        Key::Char(c) => mode.other_key(c),
        _ => (),
    }
}

fn command_mode(key: Key, mode: &mut Mode) {
    match key {
        Key::Char('\n') => mode.command_enter(),
        Key::Char(c) => mode.command_key(c),
        Key::Backspace => mode.command_del(),
        Key::Esc => mode.command_esc(),
        _ => (),
    }
}

fn insert_mode(key: Key, mode: &mut Mode) {
    match key {
        Key::Char('\n') => mode.insert_enter(),
        Key::Char(c) => mode.insert_key(c),
        Key::Backspace => mode.insert_del(),
        Key::Esc => mode.insert_esc(),
        Key::Left => mode.move_insert(Dir::Left),
        Key::Right => mode.move_insert(Dir::Right),
        Key::Up => mode.move_insert(Dir::Up),
        Key::Down => mode.move_insert(Dir::Down),
        _ => (),
    }
}

//todo, make mode not mut and make stdout seperate
fn draw(mode: &mut Mode, cells: &Vec<csv::StringRecord>) {
    match mode.input {
        InputMode::Normal => normal_draw(mode, cells),
        InputMode::Command => command_draw(mode),
        InputMode::Insert => insert_draw(mode, cells),
        InputMode::Select => (),
    }
}

fn normal_draw(mode: &mut Mode, cells: &Vec<csv::StringRecord>) {
    if !mode.no_clear {
        write!(
            mode.stdout,
            "{}{}",
            termion::cursor::Goto(0, mode.dims.1),
            termion::clear::All,
        )
        .unwrap();

        let line = mode.position.0 as i16 - mode.offset as i16 + 1;
        if line > mode.dims.1 as i16 - 1 {
            mode.offset += 1;
        } else if line > 0 {
            mode.offset -= 1;
        }
        let line = mode.position.0 - mode.offset + 1;

        for i in 0..(mode.dims.1 - 1) {
            let n = i + mode.offset;
            let contents = cells.get(n as usize);
            let cell = match contents {
                Some(cell) => cell.get(mode.position.1 as usize),
                None => break,
            };
            let cell = match cell {
                Some(cell) => cell,
                None => "XXX",
            };

            write!(
                mode.stdout,
                "{}{:>3}: {}",
                termion::cursor::Goto(mode.margin, i + 1),
                n,
                cell,
            )
            .unwrap();
        }
        write!(
            mode.stdout,
            "{}r{} c{}{}",
            termion::cursor::Goto(1, mode.dims.0),
            mode.position.0,
            mode.position.1,
            termion::cursor::Goto(mode.margin, line),
        )
        .unwrap();
    }
}

fn insert_draw(mode: &mut Mode, cells: &Vec<csv::StringRecord>) {}
fn command_draw(mode: &mut Mode) {
    write!(
        mode.stdout,
        "{}{}:{}{}{}",
        termion::cursor::Goto(0, mode.dims.1),
        termion::clear::CurrentLine,
        mode.command,
        termion::cursor::Goto(mode.cursor.1 + 2, mode.dims.1),
        termion::cursor::Show,
    )
    .unwrap();
}

//make r replace because it doesn't do anything
