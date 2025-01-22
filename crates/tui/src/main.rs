use crate::database::DBCONNECTION;
use arboard::Clipboard;
use bookrab_core::books::{Exclude, FilterMode, Include, RootBookDir, SearchResults};
use bookrab_core::database::PgPooledConnection;
use bookrab_core::errors::BookrabError;
use config::ensure_confy_works;
use crossterm::event::{KeyEvent, KeyModifiers};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::SearcherBuilder;
use ratatui::prelude::*;
use ratatui::widgets::{ListItem, ListState, Wrap};
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    widgets::{Block, Borders, List, Paragraph},
};
use std::collections::HashSet;
use std::iter::{Cycle, Filter, Iterator};
use std::{error::Error, io};
use strum::EnumIter;
use strum::IntoEnumIterator;
use style::palette::tailwind::{BLACK, GREEN, RED, SLATE};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
mod config;
mod database;

const TEXT_FG_COLOR: Color = SLATE.c600;
const INCLUDED_FG_COLOR: Color = GREEN.c500;
const EXCLUDED_FG_COLOR: Color = RED.c500;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c300).add_modifier(Modifier::BOLD);

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

#[derive(PartialEq, EnumIter)]
enum WhereWeAre {
    Input,
    Tags,
    Include,
    Exclude,
    Nowhere,
}

struct TagItem {
    name: String,
    status: TagStatus,
}

struct TagList {
    list: Vec<TagItem>,
    state: ListState,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum TagStatus {
    Include,
    Exclude,
    None,
}

/// App holds the state of the application
struct App<'a> {
    input: Input,
    where_we_are: WhereWeAre,
    root_book_dir: RootBookDir<'a>,
    tags: TagList,
    results: Vec<SearchResults>,
    include: FilterMode,
    exclude: FilterMode,
}

impl App<'_> {
    fn new<'a>(connection: &mut PgPooledConnection) -> App {
        let root_book_dir = RootBookDir::new(ensure_confy_works(), connection);
        let tags = TagList {
            list: root_book_dir
                .all_tags()
                .unwrap()
                .into_iter()
                .map(|tag| TagItem {
                    name: tag,
                    status: TagStatus::None,
                })
                .collect(),
            state: ListState::default(),
        };
        let include = FilterMode::All;
        let exclude = FilterMode::Any;
        let results = vec![];
        App {
            input: Input::default(),
            where_we_are: WhereWeAre::Nowhere,
            root_book_dir,
            tags,
            include,
            exclude,
            results,
        }
    }

    /// Returns highlighted style if `area` matches with
    /// current `self.where_we_are`.
    /// Returns a more neutral style otherwise.
    fn highlight_if_focused(&self, area: WhereWeAre) -> Style {
        if self.where_we_are == area {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        }
    }

    /// Renders the search part of the application (left side)
    fn render_search_panel(&mut self, rect: Rect, f: &mut Frame) {
        let search_panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
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
            .style(self.highlight_if_focused(WhereWeAre::Input))
            .block(Block::default().borders(Borders::ALL).title("Query"));
        f.render_widget(input, search_panel[0]);

        let tags_vec: Vec<ListItem> = self.tags.list.iter().map(|v| ListItem::from(v)).collect();
        let tags_ui = List::new(tags_vec)
            .block(Block::default().borders(Borders::ALL).title("Tags"))
            .style(self.highlight_if_focused(WhereWeAre::Tags))
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">");

        f.render_stateful_widget(tags_ui, search_panel[1], &mut self.tags.state);

        let filter_modes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Fill(1)].as_ref())
            .split(search_panel[2]);

        f.render_widget(
            Paragraph::new(format!("{:?}", self.include))
                .block(Block::default().title("Include").borders(Borders::ALL))
                .style(self.highlight_if_focused(WhereWeAre::Include)),
            filter_modes[0],
        );
        f.render_widget(
            Paragraph::new(format!("{:?}", self.exclude))
                .block(Block::default().title("Exclude").borders(Borders::ALL))
                .style(self.highlight_if_focused(WhereWeAre::Exclude)),
            filter_modes[1],
        );

        let width = search_panel[0].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        match self.where_we_are {
            WhereWeAre::Input => f.set_cursor_position((
                search_panel[0].x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
                search_panel[0].y + 1,
            )),
            _ => {}
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
        let include = Include::from(&self.tags);
        let exclude = Exclude::from(&self.tags);
        let results = self.root_book_dir.search_by_tags(
            &include,
            &exclude,
            query.to_string(),
            searcher,
            regex_builder,
        )?;
        Ok(results)
    }
    fn update_results(&mut self) {
        self.results = self.search().unwrap();
    }

    /// Cycles through selectable items on the screen.
    fn next_position(&mut self) {
        let positions = WhereWeAre::iter()
            .filter(|pos| pos != &WhereWeAre::Nowhere)
            .cycle();
        self.cycle_position(positions);
    }

    /// See `next_position` and `previous_position`.
    fn cycle_position<T: Iterator<Item = WhereWeAre>>(&mut self, mut positions: T) {
        if self.where_we_are == WhereWeAre::Nowhere {
            self.where_we_are = positions.next().unwrap();
            return;
        }
        while let Some(position) = positions.next() {
            if position == self.where_we_are {
                self.where_we_are = positions.next().unwrap();
                return;
            }
        }
    }

    /// Cycles through selectable items on the screen in the reversed order.
    fn previous_position(&mut self) {
        let positions = WhereWeAre::iter()
            .filter(|pos| pos != &WhereWeAre::Nowhere)
            .rev()
            .cycle();
        self.cycle_position(positions);
    }

    fn select_no_tags(&mut self) {
        self.tags.state.select(None);
    }

    fn select_next_tag(&mut self) {
        self.tags.state.select_next();
    }
    fn select_previous_tag(&mut self) {
        self.tags.state.select_previous();
    }
    fn select_first_tag(&mut self) {
        self.tags.state.select_first();
    }
    fn select_last_tag(&mut self) {
        self.tags.state.select_last();
    }

    /// Changes status of selected tag in the following way
    /// None => Include => Exclude => None => ...
    fn cycle_status(&mut self) {
        if let Some(i) = self.tags.state.selected() {
            self.tags.list[i].status = match self.tags.list[i].status {
                TagStatus::None => TagStatus::Include,
                TagStatus::Include => TagStatus::Exclude,
                TagStatus::Exclude => TagStatus::None,
            }
        }
    }

    /// Changes the status of the selected tag to `status` or to [`TagStatus::None`].
    fn change_status(&mut self, status: TagStatus) {
        if let Some(i) = self.tags.state.selected() {
            self.tags.list[i].status = if self.tags.list[i].status == status {
                TagStatus::None
            } else {
                status
            }
        }
    }

    /// Copies the results in the html format.
    fn copy_results(&self) -> Result<(), arboard::Error> {
        let mut ctx = Clipboard::new()?;
        let mut html = String::new();
        for result in self.results.iter() {
            let SearchResults { title, results } = result;
            if result.results.len() > 0 {
                html = format!("{html}<div><span style=\"color: blue\">{title}</span></div>");
                for single_result in results.clone() {
                    html = format!("{html}<p>{}</p>", color_match_html(single_result))
                }
            }
        }
        Ok(ctx.set().html(html, None)?)
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    fn common_bindings(key: KeyEvent, app: &mut App) {
        if key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Esc => {
                    app.where_we_are = WhereWeAre::Nowhere;
                }
                KeyCode::Enter => {
                    app.update_results();
                }
                KeyCode::Tab => {
                    app.next_position();
                }
                _ => {}
            }
        } else if key.modifiers == KeyModifiers::SHIFT {
            match key.code {
                KeyCode::BackTab => {
                    app.previous_position();
                }
                _ => {}
            }
        } else if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Char('y') => {
                    app.copy_results().expect("Error when copying results");
                }
                _ => {}
            }
        }
    }
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Char('c') => return Ok(()),
                    _ => {}
                }
            }
            common_bindings(key, &mut app);
            match app.where_we_are {
                WhereWeAre::Input => match key.code {
                    _ => {
                        app.input.handle_event(&Event::Key(key));
                    }
                },
                WhereWeAre::Include => match key.code {
                    KeyCode::Char(' ') => match app.include {
                        FilterMode::All => app.include = FilterMode::Any,
                        FilterMode::Any => app.include = FilterMode::All,
                    },
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                WhereWeAre::Exclude => match key.code {
                    KeyCode::Char(' ') => match app.exclude {
                        FilterMode::All => app.exclude = FilterMode::Any,
                        FilterMode::Any => app.exclude = FilterMode::All,
                    },
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                WhereWeAre::Tags => match key.code {
                    KeyCode::Char(' ') => app.cycle_status(),
                    KeyCode::Char('j') | KeyCode::Down => app.select_next_tag(),
                    KeyCode::Char('k') | KeyCode::Up => app.select_previous_tag(),
                    KeyCode::Char('h') | KeyCode::Left => app.change_status(TagStatus::Exclude),
                    KeyCode::Char('l') | KeyCode::Right => app.change_status(TagStatus::Include),
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                _ => match key.code {
                    KeyCode::Char('e') => {
                        app.where_we_are = WhereWeAre::Input;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
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

/// Returns `str_match` in a [`Line`] format.
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

/// Returns `str_match` in a [`Line`] format.
/// Characters inside `[matched][/matched]` will be colored (in html).
fn color_match_html<'a>(str_match: String) -> String {
    let open = "[matched]";
    let close = "[/matched]";
    let step1 = str_match.split(close);
    let mut step2: Vec<String> = vec![];
    for st in step1 {
        let possible_pair: Vec<&str> = st.split(open).collect();
        let normal_side = String::from(possible_pair[0]); // left side is not a match
        step2.push(normal_side);
        if possible_pair.len() == 2 {
            let match_side =
                "<span style=\"color: red\">".to_owned() + possible_pair[1] + "</span>";
            step2.push(match_side);
        }
    }
    step2.into_iter().collect()
}

impl From<&TagItem> for ListItem<'_> {
    fn from(value: &TagItem) -> Self {
        let line = match value.status {
            TagStatus::None => Line::styled(format!("{}", value.name), TEXT_FG_COLOR),
            TagStatus::Include => Line::styled(format!("{}", value.name), INCLUDED_FG_COLOR),
            TagStatus::Exclude => Line::styled(format!("{}", value.name), EXCLUDED_FG_COLOR),
        };
        ListItem::new(line)
    }
}
impl From<&TagList> for Include {
    fn from(value: &TagList) -> Self {
        let included: HashSet<String> = value
            .list
            .iter()
            .filter(|v| v.status == TagStatus::Include)
            .map(|v| v.name.clone())
            .collect();
        Include {
            mode: FilterMode::All,
            tags: included,
        }
    }
}
impl From<&TagList> for Exclude {
    fn from(value: &TagList) -> Self {
        let excluded: HashSet<String> = value
            .list
            .iter()
            .filter(|v| v.status == TagStatus::Exclude)
            .map(|v| v.name.clone())
            .collect();
        Exclude {
            mode: FilterMode::Any,
            tags: excluded,
        }
    }
}
