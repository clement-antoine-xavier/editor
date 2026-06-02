use std::io;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};

#[derive(Default)]
struct Editor {
    filename: Option<String>,
    lines: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll_x: usize,
    scroll_y: usize,
    modified: bool,
    message: String,
    quit: bool,
    show_help: bool,
}

impl Editor {
    fn new() -> Self {
        Self {
            lines: vec![String::new()],
            ..Default::default()
        }
    }

    fn from_file(path: &str) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };
        Ok(Self {
            filename: Some(path.to_string()),
            lines,
            modified: false,
            ..Default::default()
        })
    }

    fn save(&mut self) -> io::Result<()> {
        if let Some(ref path) = self.filename {
            let content = self.lines.join("\n");
            std::fs::write(path, content)?;
            self.modified = false;
            self.message = format!("Saved: {}", path);
        } else {
            self.message = "No filename. Use: editor <filename>".to_string();
        }
        Ok(())
    }

    fn insert_char(&mut self, c: char) {
        if self.cursor_y >= self.lines.len() {
            self.lines.push(String::new());
        }
        let line = &mut self.lines[self.cursor_y];
        if self.cursor_x > line.len() {
            self.cursor_x = line.len();
        }
        line.insert(self.cursor_x, c);
        self.cursor_x += 1;
        self.modified = true;
    }

    fn delete_char(&mut self) {
        if self.cursor_y >= self.lines.len() {
            return;
        }
        if self.cursor_x > 0 {
            self.lines[self.cursor_y].remove(self.cursor_x - 1);
            self.cursor_x -= 1;
            self.modified = true;
        } else if self.cursor_y > 0 {
            let current = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
            self.lines[self.cursor_y].push_str(&current);
            self.modified = true;
        }
    }

    fn delete_forward(&mut self) {
        if self.cursor_y >= self.lines.len() {
            return;
        }
        let line_len = self.lines[self.cursor_y].len();
        if self.cursor_x < line_len {
            self.lines[self.cursor_y].remove(self.cursor_x);
            self.modified = true;
        } else if self.cursor_y + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_y + 1);
            self.lines[self.cursor_y].push_str(&next);
            self.modified = true;
        }
    }

    fn insert_newline(&mut self) {
        if self.cursor_y >= self.lines.len() {
            self.lines.push(String::new());
            return;
        }
        let remainder = self.lines[self.cursor_y].split_off(self.cursor_x);
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.lines.insert(self.cursor_y, remainder);
        self.modified = true;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_y < self.lines.len() {
            let line_len = self.lines[self.cursor_y].len();
            if self.cursor_x < line_len {
                self.cursor_x += 1;
            } else if self.cursor_y + 1 < self.lines.len() {
                self.cursor_y += 1;
                self.cursor_x = 0;
            }
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let line_len = self.lines[self.cursor_y].len();
            if self.cursor_x > line_len {
                self.cursor_x = line_len;
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            let line_len = self.lines[self.cursor_y].len();
            if self.cursor_x > line_len {
                self.cursor_x = line_len;
            }
        }
    }

    fn move_cursor_home(&mut self) {
        self.cursor_x = 0;
    }

    fn move_cursor_end(&mut self) {
        if self.cursor_y < self.lines.len() {
            self.cursor_x = self.lines[self.cursor_y].len();
        }
    }

    fn page_up(&mut self, page_height: usize) {
        if self.cursor_y > page_height {
            self.cursor_y -= page_height;
        } else {
            self.cursor_y = 0;
        }
        let line_len = self.lines[self.cursor_y].len();
        if self.cursor_x > line_len {
            self.cursor_x = line_len;
        }
    }

    fn page_down(&mut self, page_height: usize) {
        if self.cursor_y + page_height < self.lines.len() {
            self.cursor_y += page_height;
        } else {
            self.cursor_y = self.lines.len().saturating_sub(1);
        }
        let line_len = self.lines[self.cursor_y].len();
        if self.cursor_x > line_len {
            self.cursor_x = line_len;
        }
    }

    fn go_to_start(&mut self) {
        self.cursor_y = 0;
        self.cursor_x = 0;
    }

    fn go_to_end(&mut self) {
        if !self.lines.is_empty() {
            self.cursor_y = self.lines.len() - 1;
            self.cursor_x = self.lines[self.cursor_y].len();
        }
    }

    fn adjust_scroll(&mut self, area_height: usize, area_width: usize) {
        let visible_height = area_height.saturating_sub(1);
        let visible_width = area_width.saturating_sub(1);

        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        } else if self.cursor_y >= self.scroll_y + visible_height {
            self.scroll_y = self.cursor_y.saturating_sub(visible_height - 1);
        }

        if self.cursor_x < self.scroll_x {
            self.scroll_x = self.cursor_x;
        } else if self.cursor_x >= self.scroll_x + visible_width {
            self.scroll_x = self.cursor_x.saturating_sub(visible_width - 1);
        }
    }
}

fn draw(frame: &mut Frame, editor: &Editor) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let text_area = chunks[0];
    let status_area = chunks[1];

    let title = if let Some(ref f) = editor.filename {
        if editor.modified {
            format!("*{} - Ratatui Editor", f)
        } else {
            format!("{} - Ratatui Editor", f)
        }
    } else {
        "[New File] - Ratatui Editor".to_string()
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(text_area);

    const GUTTER_WIDTH: u16 = 5;

    let visible_lines: Vec<Line> = editor
        .lines
        .iter()
        .skip(editor.scroll_y)
        .take(inner.height as usize)
        .enumerate()
        .map(|(i, line)| {
            let line_num = editor.scroll_y + i + 1;
            let display_line = if editor.scroll_x < line.len() {
                &line[editor.scroll_x..]
            } else {
                ""
            };

            Line::from(vec![
                Span::styled(
                    format!("{:4} ", line_num),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(display_line),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(Text::from(visible_lines));
    frame.render_widget(paragraph.block(block), text_area);

    // Status bar
    let cursor_info = format!("Ln {}, Col {}", editor.cursor_y + 1, editor.cursor_x + 1);
    let mode = if editor.modified { " [MODIFIED]" } else { "" };
    let status_text = if !editor.message.is_empty() {
        format!("{}{} | {}", cursor_info, mode, editor.message)
    } else {
        format!("{}{} | Ctrl+H: Help | Ctrl+S: Save | Ctrl+Q: Quit", cursor_info, mode)
    };

    let status_bar = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Blue).fg(Color::White));
    frame.render_widget(status_bar, status_area);

    // Cursor
    if !editor.show_help {
        let rel_x = editor.cursor_x.saturating_sub(editor.scroll_x);
        let rel_y = editor.cursor_y.saturating_sub(editor.scroll_y);

        let cursor_x = inner.x + GUTTER_WIDTH + rel_x as u16;
        let cursor_y = inner.y + rel_y as u16;

        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            frame.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }

    // Help overlay
    if editor.show_help {
        let help_lines = vec![
            Line::from("Ratatui Text Editor Help"),
            Line::from(""),
            Line::from("Ctrl+S      Save file"),
            Line::from("Ctrl+Q      Quit"),
            Line::from("Ctrl+H      Toggle help"),
            Line::from(""),
            Line::from("Arrow Keys  Move cursor"),
            Line::from("Home        Start of line"),
            Line::from("End         End of line"),
            Line::from("Page Up     Page up"),
            Line::from("Page Down   Page down"),
            Line::from("Ctrl+Home   Start of file"),
            Line::from("Ctrl+End    End of file"),
            Line::from(""),
            Line::from("Enter       New line"),
            Line::from("Backspace   Delete backward"),
            Line::from("Delete      Delete forward"),
        ];

        let help_width = 32u16;
        let help_height = help_lines.len() as u16 + 2;
        let help_x = area.width.saturating_sub(help_width) / 2;
        let help_y = area.height.saturating_sub(help_height) / 2;

        let help_area = Rect::new(help_x, help_y, help_width, help_height);

        frame.render_widget(Clear, help_area);
        frame.render_widget(
            Paragraph::new(Text::from(help_lines)).block(
                Block::default().borders(Borders::ALL).title("Help"),
            ),
            help_area,
        );
    }
}

fn handle_event(editor: &mut Editor, key: KeyEvent, area_height: usize) -> io::Result<()> {
    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
        return Ok(());
    }

    match key.code {
        // Save
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.save()?;
        }
        // Quit
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.quit = true;
        }
        // Help
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.show_help = !editor.show_help;
        }
        // Navigation with Ctrl
        KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => editor.go_to_start(),
        KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => editor.go_to_end(),
        // Regular keys
        KeyCode::Char(c)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            editor.insert_char(c);
        }
        KeyCode::Enter => editor.insert_newline(),
        KeyCode::Backspace => editor.delete_char(),
        KeyCode::Delete => editor.delete_forward(),
        KeyCode::Left => editor.move_cursor_left(),
        KeyCode::Right => editor.move_cursor_right(),
        KeyCode::Up => editor.move_cursor_up(),
        KeyCode::Down => editor.move_cursor_down(),
        KeyCode::Home => editor.move_cursor_home(),
        KeyCode::End => editor.move_cursor_end(),
        KeyCode::PageUp => editor.page_up(area_height),
        KeyCode::PageDown => editor.page_down(area_height),
        _ => {}
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let mut editor = if let Some(filename) = std::env::args().nth(1) {
        Editor::from_file(&filename).unwrap_or_else(|e| {
            eprintln!("Error loading file: {}", e);
            std::process::exit(1);
        })
    } else {
        Editor::new()
    };

    ratatui::run(|terminal| {
        loop {
            let area = terminal.get_frame().area();
            let area_height = area.height.saturating_sub(2) as usize;

            editor.adjust_scroll(area_height, area.width as usize);
            terminal.draw(|frame| draw(frame, &editor))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    handle_event(&mut editor, key, area_height)?;

                    if editor.quit {
                        break Ok(());
                    }
                }
            }
        }
    })
}
