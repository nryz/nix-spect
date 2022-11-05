use clap::{arg, command};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, process::Command, time};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        let mut list = StatefulList {
            state: ListState::default(),
            items,
        };

        list.state.select(Some(0));
        list
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

enum ListOrValue {
    List(Vec<String>),
    Value(String),
}

struct App {
    current_items: StatefulList<String>,
    preview_items: ListOrValue,
    current_path: String,
    current_selected: String,
    preview_path: String,
}

impl App {
    fn new(current_path: String) -> App {
        let current_items = get_completions(&current_path).unwrap();
        let current_items = StatefulList::with_items(current_items);
        let current_selected = match current_items.state.selected() {
            Some(i) => current_items.items[i].trim().to_owned(),
            None => "none".to_string(),
        };

        let mut preview_path = current_path.clone();
        preview_path.push('.');
        preview_path.push_str(&current_selected);

        let preview_items = get_completions(&preview_path).unwrap();

        if preview_items.is_empty() {
            panic!("preview_items are empty");
        }

        App {
            current_items,
            preview_items: ListOrValue::List(preview_items),
            current_path,
            current_selected,
            preview_path,
        }
    }

    fn update_current_selected(&mut self) {
        self.current_selected = match self.current_items.state.selected() {
            Some(i) => self.current_items.items[i].trim().to_owned(),
            None => "none".to_string(),
        };

        self.preview_path = self.current_path.clone();
        self.preview_path.push('.');
        self.preview_path.push_str(&self.current_selected);

        let items = get_completions(&self.preview_path).unwrap();
        if items.is_empty() {
            let value = match get_value(&self.preview_path) {
                Ok(value) => value,
                Err(error) => error,
            };

            self.preview_items = ListOrValue::Value(value);
        } else {
            self.preview_items = ListOrValue::List(items);
        }
    }

    fn previous(&mut self) {
        self.current_items.previous();
        self.update_current_selected();
    }

    fn next(&mut self) {
        self.current_items.next();
        self.update_current_selected();
    }

    fn step_in(&mut self) {
        if let ListOrValue::List(list) = &self.preview_items {
            if list.is_empty() {
                return;
            }

            self.current_path.push('.');
            self.current_path.push_str(&self.current_selected);

            self.current_items = StatefulList::with_items(list.clone());
            self.update_current_selected();
        }
    }

    fn step_out(&mut self) {
        let mut suffix = '.'.to_string();
        suffix.push_str(self.current_path.split('.').last().unwrap());

        if let Some(string) = self.current_path.strip_suffix(&suffix) {
            self.current_path = string.to_string();
        }

        self.current_items = StatefulList::with_items(get_completions(&self.current_path).unwrap());
        self.update_current_selected();
    }
}

fn get_value(path: &str) -> Result<String, String> {
    let path = path.to_owned();

    let result = Command::new("nix")
        .arg("eval")
        .arg(&path)
        .output()
        .expect("command failed to run");

    if !result.status.success() {
        let error = String::from_utf8(result.stderr).unwrap();
        return Err(format!("Error: {}", error));
    }

    Ok(String::from_utf8(result.stdout).unwrap())
}

fn get_completions(path: &str) -> Result<Vec<String>, String> {
    let mut path = path.to_owned();

    if !path.ends_with('.') && !path.ends_with('#') {
        path.push('.');
    }

    let result = Command::new("nix")
        .arg("eval")
        .arg("--raw")
        .arg(&path)
        .env("NIX_GET_COMPLETIONS", "3")
        .output()
        .expect("command failed to run");

    if !result.status.success() {
        let error = String::from_utf8(result.stderr).unwrap();
        return Err(format!("Error: {}", error));
    }

    let out = String::from_utf8(result.stdout).unwrap();

    let mut completions: Vec<String> = Vec::new();
    for s in out.lines() {
        if !s.eq("attrs") {
            completions.push(s.to_string().replace(&path, ""));
        }
    }

    Ok(completions)
}

fn render<B: Backend>(frame: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(frame.size());

    let current_items: Vec<ListItem> = app
        .current_items
        .items
        .iter()
        .map(|i| ListItem::new(Text::raw(i)).style(Style::default()))
        .collect();

    let mut path_title = app.current_path.clone();
    path_title = path_title.split('#').last().unwrap().to_string();

    let current_list = List::new(current_items)
        .block(Block::default().borders(Borders::ALL).title(path_title))
        .highlight_style(
            Style::default()
                .bg(Color::Black)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    match &app.preview_items {
        ListOrValue::List(list) => {
            let preview_items: Vec<ListItem> = list
                .iter()
                .map(|i| ListItem::new(Text::raw(i)).style(Style::default().fg(Color::Green)))
                .collect();

            let preview = List::new(preview_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(&*app.current_selected),
            );

            frame.render_widget(preview, chunks[1]);
        }
        ListOrValue::Value(value) => {
            let preview = Paragraph::new(value.clone())
                .style(Style::default())
                .block(Block::default());

            frame.render_widget(preview, chunks[1]);
        }
    };

    frame.render_stateful_widget(current_list, chunks[0], &mut app.current_items.state);
}

fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app))?;

        if event::poll(time::Duration::from_millis(1000))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Char('j') => app.next(),
                    KeyCode::Char('k') => app.previous(),
                    KeyCode::Char('h') => app.step_out(),
                    KeyCode::Char('l') => app.step_in(),
                    _ => (),
                }
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    let matches = command!()
        .arg(arg!([flake] "flake path").required(true))
        .get_matches();

    let mut path = matches
        .get_one::<String>("flake")
        .expect("expected a valid flake path")
        .to_owned();

    if path.ends_with('.') {
        panic!("flake path ends with .");
    }

    if !path.contains('#') {
        path.push('#');
    }

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let mut app = App::new(path);
    let res = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
