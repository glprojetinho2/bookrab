use crate::database::DBCONNECTION;
use bookrab_core::books::{Exclude, FilterMode, Include, RootBookDir, SearchResults};
use bookrab_core::database::PgPooledConnection;
use bookrab_core::errors::BookrabError;
use config::ensure_confy_works;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::SearcherBuilder;
use ratatui::prelude::*;
use ratatui::widgets::Wrap;
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    widgets::{Block, Borders, List, Paragraph},
};
use std::collections::HashSet;
use std::{error::Error, io};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
mod config;
mod database;

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let connection = &mut DBCONNECTION.get().unwrap();

    // create app and run it
    let app = App::new(connection);
    let res = run_app(&mut terminal, app);

    // restore terminal
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

enum InputMode {
    Normal,
    Editing,
}

/// App holds the state of the application
struct App<'a> {
    input: Input,
    input_mode: InputMode,
    root_book_dir: RootBookDir<'a>,
    tags: HashSet<String>,
    results: Vec<SearchResults>,
    include: Include,
    exclude: Exclude,
}

impl App<'_> {
    fn new<'a>(connection: &mut PgPooledConnection) -> App {
        let root_book_dir = RootBookDir::new(ensure_confy_works(), connection);
        let tags = root_book_dir.all_tags().unwrap();
        let include = Include {
            mode: FilterMode::All,
            tags: HashSet::new(),
        };
        let exclude = Exclude {
            mode: FilterMode::Any,
            tags: HashSet::new(),
        };
        let results = vec![];
        App {
            input: Input::default(),
            input_mode: InputMode::Normal,
            root_book_dir,
            tags,
            include,
            exclude,
            results,
        }
    }

    /// Renders the search part of the application (left side)
    fn render_search_panel(&self, rect: Rect, f: &mut Frame) {
        let search_panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(rect);
        // let help = Paragraph::new(format!("{:?}", ensure_confy_works().book_path));
        // f.render_widget(help, search_panel[0]);
        let input = Paragraph::new(self.input.value())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title("Query"));
        f.render_widget(input, search_panel[1]);

        let tags_ui = List::new(self.tags.clone())
            .block(Block::default().borders(Borders::ALL).title("Tags"));

        f.render_widget(tags_ui, search_panel[2]);

        let filter_modes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Fill(1)].as_ref())
            .split(search_panel[3]);
        f.render_widget(
            Paragraph::new(format!("{:?}", self.include.mode))
                .block(Block::default().title("Include").borders(Borders::ALL)),
            filter_modes[0],
        );
        f.render_widget(
            Paragraph::new(format!("{:?}", self.exclude.mode))
                .block(Block::default().title("Exclude").borders(Borders::ALL)),
            filter_modes[1],
        );

        let width = search_panel[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        match self.input_mode {
            InputMode::Normal => {}

            InputMode::Editing => f.set_cursor_position((
                search_panel[1].x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
                search_panel[1].y + 1,
            )),
        }
    }

    /// Renders the search results part of the application (right side)
    fn render_result_panel(&mut self, rect: Rect, f: &mut Frame) {
        //TODO: remover unwraps
        let result_panel = Layout::default()
            .constraints([Constraint::Fill(1)].as_ref())
            .split(rect);
        let mut result_text: Vec<Line> = vec![];
        for result in self.results.iter() {
            let SearchResults { title, results } = result;
            if results.len() > 0 {
                result_text.push(Span::from(title).blue().into());
                for result_contents in results {
                    let colored_result = color_match(&result_contents);
                    result_text.push(colored_result.into());
                }
            }
        }
        let result_ui = Paragraph::new(Text::from(result_text));
        f.render_widget(
            result_ui
                .wrap(Wrap { trim: true })
                .block(Block::new().borders(Borders::ALL).title("Results")),
            result_panel[0],
        );
    }

    fn search(&mut self) -> Result<Vec<SearchResults>, BookrabError> {
        let query = self.input.value();
        let searcher = SearcherBuilder::new().build();
        let regex_builder = RegexMatcherBuilder::new();
        let results = self.root_book_dir.search_by_tags(
            &self.include,
            &self.exclude,
            query.to_string(),
            searcher,
            regex_builder,
        )?;
        Ok(results)
    }
    fn update_results(&mut self) {
        self.results = self.search().unwrap();
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        app.update_results();
                        app.input.reset();
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {
                        app.input.handle_event(&Event::Key(key));
                    }
                },
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let two_panels = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(f.area());
    app.render_search_panel(two_panels[0], f);
    app.render_result_panel(two_panels[1], f);
}

/// Returns `str_match` in a `Text` format.
/// Characters inside `[matched][/matched]` will be colored.
fn color_match<'a>(str_match: &'a str) -> Line<'a> {
    let open = "[matched]";
    let close = "[/matched]";
    let step1 = str_match.split(close);
    let mut step2: Vec<Span> = vec![];
    for st in step1 {
        let possible_pair: Vec<&str> = st.split(open).collect();
        let normal_side = Span::from(possible_pair[0]); // left side is not a match
        step2.push(normal_side);
        if possible_pair.len() == 2 {
            let match_side = Span::styled(possible_pair[1], Color::Red);
            step2.push(match_side);
        }
    }
    Line::from(step2)
}
