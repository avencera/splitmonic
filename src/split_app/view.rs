use crate::{
    split_app::{InputMode, Screen, SplitApp},
    Backend,
};

use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw(app: &mut SplitApp, frame: &mut Frame<Backend>) {
    let help_box_size = match &app.screen {
        Screen::List => 3,
        _ => 1,
    };

    // setup layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Min(help_box_size + 1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(frame.size());

    // render blocks
    frame.render_widget(help_message_block(&app), chunks[0]);
    frame.render_widget(input_block(&app), chunks[1]);

    match app.screen {
        Screen::List => {}

        Screen::Input(InputMode::Normal) => {}

        Screen::Input(InputMode::Editing) => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            frame.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            )
        }
    }

    // We can now render the item list
    let mnemonic_block = mnemonic_block(&app);
    frame.render_stateful_widget(mnemonic_block, chunks[2], &mut app.mnemonic.state);
}

fn help_message_block(app: &SplitApp) -> Paragraph {
    let (mut text, style) = match app.screen {
        Screen::Input(InputMode::Normal) => (
            Text::from(Spans::from(vec![
                Span::raw("Press "),
                Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to exit, "),
                Span::styled("i ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to start editing, "),
                Span::styled("↓ ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("or "),
                Span::styled("<TAB> ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to access the word list"),
            ])),
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),

        Screen::Input(InputMode::Editing) => (
            Text::from(Spans::from(vec![
                Span::raw("Press "),
                Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to add the word, "),
                Span::styled("↓ ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to access the word list, "),
                Span::styled("<TAB> ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to see the next autocomplete word"),
            ])),
            Style::default(),
        ),

        Screen::List => (
            {
                let mut texts = Text::from(Spans::from(vec![
                    Span::raw("Press "),
                    Span::styled("<TAB> ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to go to normal mode, "),
                    Span::styled("i ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to add new words, "),
                ]));

                texts.extend(Text::from(Spans::from(vec![
                    Span::styled(
                        "      <ALT> + ↓ ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("to move word down, "),
                    Span::styled("<ALT> + ↑ ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to move word up, "),
                ])));

                texts.extend(Text::from(Spans::from(vec![
                    Span::styled("      d ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to delete word, "),
                    Span::styled("e ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to edit word "),
                ])));

                texts
            },
            Style::default(),
        ),
    };

    text.patch_style(style);
    Paragraph::new(text)
}

fn input_block(app: &SplitApp) -> Paragraph {
    let input_text = match app.screen {
        Screen::Input(InputMode::Editing) => {
            let autocomplete = if app.autocomplete.len() >= app.input.len() {
                &app.autocomplete[app.input.len()..]
            } else {
                &app.autocomplete
            };

            vec![Spans::from(vec![
                Span::raw(&app.input),
                Span::styled(autocomplete, Style::default().fg(Color::DarkGray)),
            ])]
        }
        _ => vec![Spans::from(Span::raw(""))],
    };

    Paragraph::new(input_text)
        .style(match app.screen {
            Screen::Input(InputMode::Editing) => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"))
}

fn mnemonic_block<'a, 'b>(app: &'a SplitApp) -> List<'b> {
    let messages: Vec<ListItem> = app
        .mnemonic
        .items
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", i + 1, m)))];
            ListItem::new(content)
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    List::new(messages)
        .style(Style::default())
        .block(match app.screen {
            Screen::List => Block::default()
                .borders(Borders::ALL)
                .title("Mnemonic")
                .border_style(Style::default().fg(Color::Yellow)),

            _ => Block::default()
                .borders(Borders::ALL)
                .title("Mnemonic")
                .border_style(Style::default()),
        })
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        )
        .highlight_symbol("> ")
}
