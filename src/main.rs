use std::collections::HashSet;
use std::io;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use clap::Parser;
use walkdir::WalkDir;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use regex::RegexBuilder;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use num_cpus;

#[derive(Parser)]
struct Args {
    #[arg(default_value = "/")]
    path: String,
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    let mut search_path = args.path;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Selección de directorio
    {
        let mut input_path = search_path.clone();
        let mut cursor_pos_dir: usize = input_path.len();

        loop {
            terminal.draw(|f| {
                let size = f.size();
                let rect = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
                    .margin(2)
                    .split(size)[0];

                let mut display_path = input_path.clone();
                if cursor_pos_dir <= display_path.len() {
                    display_path.insert(cursor_pos_dir, '|');
                }

                let block = Paragraph::new(display_path)
                    .block(Block::default().borders(Borders::ALL).title(" Directorio donde buscar: "));
                f.render_widget(block, rect);
            })?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char(c) => {
                            input_path.insert(cursor_pos_dir, c);
                            cursor_pos_dir += 1;
                        }
                        KeyCode::Backspace => {
                            if cursor_pos_dir > 0 {
                                input_path.remove(cursor_pos_dir - 1);
                                cursor_pos_dir -= 1;
                            }
                        }
                        KeyCode::Delete => {
                            if cursor_pos_dir < input_path.len() {
                                input_path.remove(cursor_pos_dir);
                            }
                        }
                        KeyCode::Left => {
                            if cursor_pos_dir > 0 { cursor_pos_dir -= 1; }
                        }
                        KeyCode::Right => {
                            if cursor_pos_dir < input_path.len() { cursor_pos_dir += 1; }
                        }
                        KeyCode::Enter => {
                            if !input_path.is_empty() {
                                search_path = input_path.clone();
                                break;
                            }
                        }
                        KeyCode::Esc => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let query = Arc::new(Mutex::new(String::new()));
    let results: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let result_set: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let (query_tx, query_rx) = mpsc::channel::<String>();
    let (result_tx, result_rx) = mpsc::channel::<String>();

    // Cache global de archivos
    let file_cache: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let file_cache = Arc::clone(&file_cache);
        let search_path = search_path.clone();
        thread::spawn(move || {
            let pool = ThreadPoolBuilder::new()
                .num_threads(num_cpus::get())
                .build()
                .unwrap();

            pool.install(|| {
                WalkDir::new(&search_path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .par_bridge()
                    .filter(|e| e.file_type().is_file())
                    .for_each(|e| {
                        let path = e.path().display().to_string();
                        file_cache.lock().unwrap().push(path);
                    });
            });
        });
    }

    // Hilo de búsqueda con soporte de comodines y palabra completa
    {
        let result_tx = result_tx.clone();
        let file_cache = Arc::clone(&file_cache);
        thread::spawn(move || {
            while let Ok(q) = query_rx.recv() {
                let q = q.to_lowercase();
                if q.is_empty() { continue; }

                let snapshot: Vec<String> = { file_cache.lock().unwrap().clone() };

                let pattern = if q.contains('*') || q.contains('?') {
                    let mut s = regex::escape(&q);
                    s = s.replace(r"\*", ".*"); // 0 o más caracteres
                    s = s.replace(r"\?", "."); // ? : 1 caracter ; ?? : 2 caracteres ; ??? : 3 caracteres ; etc.
                    format!(r"^{}$", s)
                } else {
                    format!(r"\b{}\b", regex::escape(&q)) // Coincidencia exacta de palabra
                };

                let re = RegexBuilder::new(&pattern)
                    .case_insensitive(true)
                    .build()
                    .unwrap();

                snapshot.par_iter()
                    .filter(|path| {
                        let fname = std::path::Path::new(path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();
                        re.is_match(&fname)
                    })
                    .for_each(|path| { result_tx.send(path.clone()).ok(); });
            }
        });
    }

    let debounce_duration = Duration::from_millis(300);
    let mut last_input = Instant::now();
    let mut cursor_pos: usize = 0;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
                .split(size);

            let q = query.lock().unwrap();
            let mut display_q = q.clone();
            if cursor_pos <= display_q.len() {
                display_q.insert(cursor_pos, '|');
            }
            let search_block = Paragraph::new(display_q)
                .block(Block::default().borders(Borders::ALL).title(" Buscar: "));
            f.render_widget(search_block, chunks[0]);
            drop(q);

            let res = results.lock().unwrap();
            let items: Vec<ListItem> = res.iter().map(|r| ListItem::new(r.clone())).collect();
            let list_block = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Resultados ⤵ "))
                .highlight_style(Style::default().bg(Color::Blue));
            f.render_stateful_widget(list_block, chunks[1], &mut list_state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let mut q = query.lock().unwrap();
                match key.code {
                    KeyCode::Char(c) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' { break; }
                        q.insert(cursor_pos, c);
                        cursor_pos += 1;
                        last_input = Instant::now();
                    }
                    KeyCode::Backspace => {
                        if cursor_pos > 0 {
                            q.remove(cursor_pos - 1);
                            cursor_pos -= 1;
                            last_input = Instant::now();
                        }
                    }
                    KeyCode::Delete => {
                        if cursor_pos < q.len() {
                            q.remove(cursor_pos);
                            last_input = Instant::now();
                        }
                    }
                    KeyCode::Left => {
                        if cursor_pos > 0 { cursor_pos -= 1; }
                    }
                    KeyCode::Right => {
                        if cursor_pos < q.len() { cursor_pos += 1; }
                    }
                    KeyCode::Esc => break,
                    KeyCode::Down => {
                        let len = results.lock().unwrap().len();
                        let i = list_state.selected().unwrap_or(0);
                        if i + 1 < len { list_state.select(Some(i + 1)); }
                    }
                    KeyCode::Up => {
                        let i = list_state.selected().unwrap_or(0);
                        if i > 0 { list_state.select(Some(i - 1)); }
                    }
                    KeyCode::Enter => {
                        if let Some(i) = list_state.selected() {
                            if let Some(path) = results.lock().unwrap().get(i) {
                                let path = path.clone();
                                thread::spawn(move || {
                                    #[cfg(target_os = "linux")] let _ = std::process::Command::new("xdg-open").arg(path).spawn();
                                    #[cfg(target_os = "windows")] let _ = std::process::Command::new("cmd").arg("/C").arg("start").arg(path).spawn();
                                    #[cfg(target_os = "macos")] let _ = std::process::Command::new("open").arg(path).spawn();
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_input.elapsed() >= debounce_duration {
            let current_query = query.lock().unwrap().clone();
            results.lock().unwrap().clear();
            result_set.lock().unwrap().clear();
            query_tx.send(current_query).ok();
            last_input = Instant::now() + Duration::from_secs(3600);
        }

        for partial in result_rx.try_iter() {
            let mut set = result_set.lock().unwrap();
            if set.insert(partial.clone()) {
                results.lock().unwrap().push(partial);
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}