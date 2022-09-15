//tutorial-read-01.rs
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_macros)]
#![allow(unused_imports)]

use std::io::{stdin, stdout, Stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use std::cmp;
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
fn write(cells: &Vec<csv::StringRecord>) {
    let file_path = get_first_arg().unwrap();
    let file = File::create(file_path).unwrap();
    let mut wrt = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_writer(file);
    for result in cells {
        wrt.write_record(result.iter()).unwrap();
    }
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

#[derive(PartialEq)]
enum ViewMode {
    Column,
    Row,
}

#[derive(PartialEq)]
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
    position: (u16, u16), //column, row
    offset: u16,
    minimap: bool,
    minimap_width: u16,
    minimap_height: u16,
    dims: (u16, u16),
    margin: u16,
    paste: String,
    command: String,
    stdout: termion::screen::AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    stale: bool,
    cursor: (u16, u16),
    exit: bool,
    no_clear: bool,
    mode_pos: (u16, u16),
    cells: Vec<csv::StringRecord>,
    cols: u16,
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
        let (ver, hor) = match self.view {
            ViewMode::Column => (&mut self.position.1, &mut self.position.0),
            ViewMode::Row => (&mut self.position.0, &mut self.position.1),
        };
        match dir {
            Dir::Left => *hor += 1,
            Dir::Right => {
                if *hor > 0 {
                    *hor -= 1;
                }
            }
            Dir::Up => {
                if *ver > 0 {
                    *ver -= 1;
                }
            }
            Dir::Down => *ver += 1,
        }
    }
    fn enter_command(&mut self) {
        self.input = InputMode::Command;
        self.command.clear();
        self.cursor = (0, 0);
    }
    fn edit_cell(&mut self) {
        self.input = InputMode::Insert;
        self.command = get_cell(self);
        self.cursor = (0, 0);
    }
    fn append_cell(&mut self) {
        self.input = InputMode::Insert;
        self.command = get_cell(self);
        self.cursor = (self.command.len() as u16, 0);
    }
    fn change_cell(&mut self) {
        self.input = InputMode::Insert;
        self.command.clear();
        self.cursor = (0, 0);
    }
    fn delete_cell(&mut self) {
        self.paste = get_cell(self);
        set_cell("", self);
    }
    fn yank_cell(&mut self) {
        self.paste = get_cell(self);
    }
    fn paste_cell(&mut self) {
        set_cell(self.paste.clone().as_str(), self);
    }
    fn view_toggle(&mut self) {
        self.view = match self.view {
            ViewMode::Column => ViewMode::Row,
            ViewMode::Row => ViewMode::Column,
        };
    }
    fn minimap_toggle(&mut self) {
        self.minimap = !self.minimap;
    }
    fn command_key(&mut self, c: char) {
        self.command.insert(self.cursor.0 as usize, c);
        self.cursor.0 += 1
    }
    fn command_del(&mut self) {
        if self.cursor.0 > 0 {
            self.command.remove(self.cursor.0 as usize - 1);
            self.cursor.0 -= 1;
        }
    }
    fn command_enter(&mut self) {
        self.input = InputMode::Normal;
        command_parse(self.command.clone().as_str(), self)
    }
    fn command_esc(&mut self) {
        self.input = InputMode::Normal;
    }
    fn insert_key(&mut self, c: char) {
        self.command.insert(self.cursor.0 as usize, c);
        self.cursor.0 += 1
    }
    fn insert_enter(&mut self) {
        self.input = InputMode::Normal;
        set_cell(self.command.clone().as_str(), self);
    }
    fn insert_esc(&mut self) {
        self.input = InputMode::Normal;
    }
    fn insert_del(&mut self) {
        if self.cursor.0 > 0 {
            self.command.remove(self.cursor.0 as usize - 1);
            self.cursor.0 -= 1;
        }
    }
    fn move_insert(&mut self, dir: Dir) {
        match dir {
            Dir::Left => {
                if self.cursor.0 > 0 {
                    self.cursor.0 -= 1
                }
            }
            Dir::Right => {
                if self.cursor.0 < self.command.len() as u16 {
                    self.cursor.0 += 1
                }
            }
            _ => (),
        };
    }
}

fn get_cell(mode: &mut Mode) -> String {
    let contents = mode.cells.get(mode.position.1 as usize);
    match contents {
        Some(cell) => match cell.get(mode.position.0 as usize) {
            Some(cell) => cell.to_string(),
            None => "".to_string(),
        },
        None => "".to_string(),
    }
}

fn set_cell(txt: &str, mode: &mut Mode) {
    let contents = mode.cells.get_mut(mode.position.1 as usize);
    let min = std::cmp::max(mode.position.0 + 1, mode.cols);
    let empty = csv::StringRecord::from(vec![""].repeat(min as usize));
    match contents {
        Some(cell) => match cell.get(mode.position.0 as usize) {
            Some(_) => {
                let new = mod_r(&cell, mode.position.0 as usize, txt);
                cell.clear();
                cell.extend(new.iter());
            }
            None => {
                cell.extend(vec![""].repeat(mode.position.0 as usize - cell.len()));
                cell.push_field(txt);
            }
        },
        None => {
            mode.cells.resize(mode.position.1 as usize, empty.clone());
            mode.cells
                .push(mod_r(&empty, mode.position.0 as usize, txt));
        }
    };
}

fn mod_r(base: &csv::StringRecord, x: usize, txt: &str) -> csv::StringRecord {
    base.into_iter()
        .enumerate()
        .map(|(i, v)| if i == x { txt } else { v })
        .collect()
}

fn command_parse(cmd: &str, mode: &mut Mode) {
    let mut args = cmd.split_ascii_whitespace();
    match args.next() {
        Some("q") => mode.exit = true,
        Some("w") => write(&mode.cells),
        Some("set") => match args.next() {
            Some("minimap") => match args.next() {
                Some("width") => {
                    mode.minimap_width = match args.next() {
                        Some(c) => c.parse().unwrap_or(mode.minimap_width),
                        _ => 10,
                    }
                }
                Some("height") => {
                    mode.minimap_height = match args.next() {
                        Some(c) => c.parse().unwrap_or(mode.minimap_height),
                        _ => 6,
                    }
                }
                None => mode.minimap = !mode.minimap,
                _ => (),
            },
            _ => (),
        },
        Some("gr") => {
            mode.position.1 = match args.next() {
                Some(c) => c.parse().unwrap_or(mode.position.1),
                _ => 0,
            }
        }
        Some("gc") => {
            mode.position.0 = match args.next() {
                Some(c) => c.parse().unwrap_or(mode.position.0),
                _ => 0,
            }
        }
        _ => (),
    }
}

//PROBLEM: terminal cannot get keyup requests
//this means no chorded keybinds
fn main() {
    //setup input output
    //let ttyin = get_tty().unwrap();
    let cells = run().unwrap();
    let cols = match cells.get(0) {
        Some(cell) => cell.len() as u16,
        None => 0,
    };

    let ttyin = stdin();
    let ttyout = termion::screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());
    let mut mode = Mode {
        view: ViewMode::Column,
        input: InputMode::Normal,
        position: (0, 0),
        offset: 0,
        minimap: true,
        minimap_width: 6,
        minimap_height: 6,
        dims: termion::terminal_size().unwrap(),
        margin: 0,
        paste: String::from(""),
        command: String::from(""),
        stdout: ttyout,
        stale: true,
        cursor: (0, 0),
        exit: false,
        no_clear: false,
        mode_pos: (0, 0),
        cells: cells,
        cols: cols,
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
    draw(&mut mode);
    mode.stdout.flush().unwrap();
    for c in ttyin.keys() {
        mode.no_clear = false;
        key_delegate(c.unwrap(), &mut mode); // Print the key we type...
        draw(&mut mode);
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
        Key::Char('a') => mode.append_cell(),
        Key::Char('c') => mode.change_cell(),
        Key::Char('d') => mode.delete_cell(),
        Key::Char('y') => mode.yank_cell(),
        Key::Char('p') => mode.paste_cell(),

        Key::Char('t') => mode.view_toggle(),
        Key::Char('m') => mode.minimap_toggle(),

        Key::Char('g') => {
            mode.enter_command();
            mode.command = "gc ".to_string();
            mode.cursor.0 = mode.command.len() as u16;
        }
        Key::Char('G') => {
            mode.enter_command();
            mode.command = "gr ".to_string();
            mode.cursor.0 = mode.command.len() as u16;
        }

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
        Key::Left => mode.move_insert(Dir::Left),
        Key::Right => mode.move_insert(Dir::Right),
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
fn draw(mode: &mut Mode) {
    match mode.input {
        InputMode::Normal => normal_draw(mode),
        InputMode::Command => command_draw(mode),
        InputMode::Insert => normal_draw(mode),
        InputMode::Select => (),
    }
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

fn valid(cells: &Vec<csv::StringRecord>, x: u16, y: u16) -> bool {
    let contents = cells.get(y as usize);
    return match contents {
        Some(cell) => cell.get(x as usize).is_some(),
        None => false,
    };
}

fn normal_draw(mode: &mut Mode) {
    let norm = format!(
        "{}{}",
        termion::color::White.fg_str(),
        termion::color::Black.bg_str()
    );
    let red = format!(
        "{}{}",
        termion::color::Red.fg_str(),
        termion::color::Black.bg_str()
    );
    let invr = format!(
        "{}{}",
        termion::color::Black.fg_str(),
        termion::color::AnsiValue::grayscale(23).bg_string()
    );
    let dull = format!(
        "{}{}",
        termion::color::AnsiValue::grayscale(15).fg_string(),
        termion::color::Black.bg_str(),
    );
    write!(mode.stdout, "{}", termion::cursor::Hide,).unwrap();
    if mode.minimap {
        //TODO empty cells are greyed out
        let width = mode.minimap_width * 2 + 3;
        mode.margin = width - 1;
        let pos = &mut mode.mode_pos;
        if mode.position.0 < pos.0 {
            pos.0 = cmp::max(pos.0, mode.minimap_width) - mode.minimap_width;
        }
        if mode.position.1 < pos.1 {
            pos.1 = cmp::max(pos.1, mode.minimap_height) - mode.minimap_height;
        }
        if mode.position.0 >= pos.0 + mode.minimap_width {
            pos.0 += mode.minimap_width;
        }
        if mode.position.1 >= pos.1 + mode.minimap_height {
            pos.1 += mode.minimap_height;
        }

        for y in 1..(mode.minimap_height + 1) {
            let mut line = String::new();
            let a = "⊠"; //"▦"; //"✚"; //"⏺"; //"▣";
            let b = "▢";
            for x in 0..mode.minimap_width {
                let xx = pos.0 + x;
                let yy = pos.1 + y - 1;
                if (xx, yy) == mode.position {
                    line.push_str(invr.as_str());
                    line.push_str(a);
                } else if ( xx == mode.position.0 && mode.view == ViewMode::Column )
                       || ( yy == mode.position.1 && mode.view == ViewMode::Row )
                {
                    line.push_str(invr.as_str());
                    line.push('⏹');
                } else {
                    if !valid(&mode.cells, xx, yy) {
                        line.push_str(dull.as_str());
                    }
                    if (xx, yy) == mode.position {
                        line.push_str(a);
                    } else {
                        line.push_str(b);
                    }
                }
                if x == mode.minimap_width - 1 || mode.view == ViewMode::Column {
                    line.push_str(norm.as_str());
                }
                line.push(' ');
                line.push_str(norm.as_str());
            }
            write!(mode.stdout, "{}{} ", termion::cursor::Goto(2, y + 2), line).unwrap();
        }

        write!(
            mode.stdout,
            "{}└{}┌─{:<len$}{}│┌{:<len$}{}{:>len$}┘│{}┐{}{:>len$}─┘",
            termion::cursor::Goto(1, 3),
            termion::cursor::Goto(1, 1),
            pos.1,
            termion::cursor::Goto(1, 2),
            pos.0,
            termion::cursor::Goto(1, mode.minimap_height + 3),
            pos.0 + mode.minimap_width - 1,
            termion::cursor::Goto(width - 2, mode.minimap_height + 2),
            termion::cursor::Goto(1, mode.minimap_height + 4),
            pos.1 + mode.minimap_height - 1,
            len = width as usize - 4,
        )
        .unwrap();
        for y in (mode.minimap_height + 4)..(mode.dims.1 - 1) {
            write!(
                mode.stdout,
                "{}{}",
                termion::cursor::Goto(1, y + 1),
                " ".repeat(width as usize)
            )
            .unwrap();
        }
    } else {
        mode.margin = 0;
    }

    if !mode.no_clear {
        let down = match mode.view {
            ViewMode::Column => mode.position.1,
            ViewMode::Row => mode.position.0,
        };

        if down < mode.offset {
            mode.offset = down
        }

        if down - mode.offset > mode.dims.1 - 2 {
            mode.offset = down + 2 - mode.dims.1;
        }

        let line = down - mode.offset;

        for i in 0..(mode.dims.1 - 1) {
            let n = match mode.view {
                ViewMode::Column => i + mode.offset,
                ViewMode::Row => mode.position.1,
            };
            let m = match mode.view {
                ViewMode::Row => i + mode.offset,
                ViewMode::Column => mode.position.0,
            };

            let active = (m, n) == mode.position;
            let contents = mode.cells.get(n as usize);
            let (cell, invalid) = match contents {
                Some(cell) => match cell.get(m as usize) {
                    Some(cell) => (cell.to_string(), false),
                    None => ("¶".to_string(), true),
                },
                None => ("¶".to_string(), false),
            };

            let mlen = mode.dims.0 - mode.margin - 5;
            let cell = if active && mode.input == InputMode::Insert {
                mode.command.clone()
            } else {
                cell
            };

            let cell = truncate(cell.as_str(), mlen as usize);
            let cell = format!("{:<len$}", cell, len = mlen as usize);
            let color = match &mode.input {
                InputMode::Normal => {
                    if active {
                        &invr
                    } else if invalid {
                        &red
                    } else {
                        &norm
                    }
                }
                _ => {
                    if active {
                        &norm
                    } else {
                        &dull
                    }
                }
            };
            let cell = format!("{}{}{}", color, cell, norm);
            let l = match mode.view {
                ViewMode::Row => m,
                ViewMode::Column => n,
            };
            write!(
                mode.stdout,
                "{} {:>3}: {}",
                termion::cursor::Goto(mode.margin, i + 1),
                l,
                cell
            )
            .unwrap();
        }
        write!(
            mode.stdout,
            "{}r{} c{}{}{}{}",
            termion::cursor::Goto(1, mode.dims.1),
            mode.position.1,
            mode.position.0,
            termion::clear::UntilNewline,
            termion::cursor::Goto(mode.margin + 6 + mode.cursor.0, line + 1),
            if true {
                termion::cursor::Show.to_string()
            } else {
                termion::cursor::Hide.to_string()
            },
        )
        .unwrap();
    }
}

fn command_draw(mode: &mut Mode) {
    write!(
        mode.stdout,
        "{}{}:{}{}{}",
        termion::cursor::Goto(1, mode.dims.1),
        termion::clear::CurrentLine,
        mode.command,
        termion::cursor::Goto(mode.cursor.0 + 2, mode.dims.1),
        termion::cursor::Show,
    )
    .unwrap();
}

//make r replace because it doesn't do anything
