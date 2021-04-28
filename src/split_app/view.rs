use crate::{
    split_app::{InputMode, Screen, SplitApp},
    ui::util::stateful_list::StatefulList,
    Backend,
};

use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw(app: &mut SplitApp, frame: &mut Frame<Backend>) {
    let help_box_size = match &app.screen {
        Screen::List => 4,
        Screen::PhraseList(_) => 4,
        _ => 1,
    };

    let input_box_size = 3;

    // setup layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(help_box_size + 1),
                Constraint::Length(input_box_size),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(frame.size());

    // render blocks
    frame.render_widget(help_message_block(&app), chunks[0]);

    // conditionally render input_block
    match app.screen {
        Screen::SaveLocationInput => {}
        _ => frame.render_widget(input_block(&app), chunks[1]),
    };

    // cursor handling
    match app.screen {
        Screen::List => {}
        Screen::PhraseList(_) => {}
        Screen::WordInput(InputMode::Normal) => {}
        Screen::WordInput(InputMode::Inserting) | Screen::WordInput(InputMode::Editing(_)) => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            frame.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            )
        }
        Screen::SaveLocationInput => frame.set_cursor(
            chunks[3].x + app.save_location.width() as u16 + 1,
            chunks[3].y + 1,
        ),
    }

    let main_sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(67)].as_ref())
        .split(chunks[2]);

    // We can now render the item list
    let mnemonic_block = mnemonic_block(&app);
    frame.render_stateful_widget(mnemonic_block, main_sections[0], &mut app.mnemonic.state);

    render_phrases_blocks(app, frame, &main_sections);

    frame.render_widget(save_area(&app), chunks[3]);

    frame.render_widget(messages_area(&app), chunks[4])
}

fn help_message_block(app: &SplitApp) -> Paragraph {
    let (mut text, style) = match app.screen {
        Screen::WordInput(InputMode::Normal) => (
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

        Screen::WordInput(InputMode::Inserting) | Screen::WordInput(InputMode::Editing(_)) => (
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

                if app.mnemonic.len() == 24 {
                    texts.extend(Text::from(Spans::from(vec![
                        Span::styled(
                            "      <ENTER> ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("to generate your split phrases"),
                    ])));
                }

                texts
            },
            Style::default(),
        ),

        Screen::PhraseList(_) => (
            {
                let mut texts = Text::from(Spans::from(vec![
                    Span::raw("Press "),
                    Span::styled("→ ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to go to the next list, "),
                    Span::styled("← ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to go to the previous list, "),
                ]));

                texts.extend(Text::from(Spans::from(vec![
                    Span::styled(
                        "        <ENTER> ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("to toggle selecting the phrases for saving"),
                ])));

                texts.extend(Text::from(Spans::from(vec![
                    Span::styled("        a ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to select/deselect all"),
                ])));

                texts.extend(Text::from(Spans::from(vec![
                    Span::styled(
                        "        <TAB> ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("to select a location to save to"),
                ])));

                texts
            },
            Style::default(),
        ),

        Screen::SaveLocationInput => (
            Text::from(Spans::from(vec![
                Span::raw("Press "),
                Span::styled("<ENTER> ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to save "),
                Span::raw(app.number_of_selected_phrases().to_string()),
                Span::raw(" phrases to the location below"),
            ])),
            Style::default(),
        ),
    };

    text.patch_style(style);
    Paragraph::new(text)
}

fn input_block(app: &SplitApp) -> Paragraph {
    let input_text = match app.screen {
        Screen::WordInput(InputMode::Inserting) | Screen::WordInput(InputMode::Editing(_)) => {
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
            Screen::WordInput(InputMode::Inserting) => Style::default().fg(Color::Yellow),
            Screen::WordInput(InputMode::Editing(_)) => Style::default().fg(Color::Yellow),
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

    let block_border_style = match (&app.screen, app.mnemonic.len()) {
        (_, 24) => Style::default().fg(Color::Green),
        (Screen::List, _) => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    };

    // Create a List from all list items and highlight the currently selected one
    List::new(messages)
        .style(Style::default())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Mnemonic")
                .border_style(block_border_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        )
        .highlight_symbol("> ")
}

fn phrase_block<'a>(
    selected: bool,
    screen: &Screen,
    phrases: &StatefulList<String>,
    index: usize,
) -> List<'a> {
    let title = format!("{} of 5", index + 1);

    let border = if selected {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let border = match screen {
        Screen::PhraseList(current) if current == &index => border.add_modifier(Modifier::BOLD),
        _ => border,
    };

    let messages: Vec<ListItem> = phrases
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        )
        .highlight_symbol("> ")
}

fn render_phrases_blocks(app: &mut SplitApp, frame: &mut Frame<Backend>, chunks: &[Rect]) {
    let phrases_sections = Layout::default()
        .direction(Direction::Horizontal)
        .horizontal_margin(1)
        .vertical_margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled("Phrases", Style::default()));

    frame.render_widget(block, chunks[1]);

    for (index, phrases) in app.phrases.iter_mut().enumerate() {
        let mblock = phrase_block(
            *app.selected_phrases.get(&index).unwrap_or(&false),
            &app.screen,
            &phrases,
            index,
        );
        frame.render_stateful_widget(mblock, phrases_sections[index], &mut phrases.state)
    }
}

fn save_area(app: &SplitApp) -> Paragraph {
    let style = match app.screen {
        Screen::SaveLocationInput => Style::default().fg(Color::Yellow),
        _ => Style::default().fg(Color::DarkGray),
    };

    let input_text = vec![Spans::from(vec![Span::raw(&app.save_location)])];

    Paragraph::new(input_text)
        .style(style.add_modifier(Modifier::RAPID_BLINK))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Save")
                .border_style(style),
        )
}

fn messages_area(app: &SplitApp) -> Paragraph {
    use crate::split_app::Message;

    let dark_gray = Style::default().fg(Color::DarkGray);
    let gray = Style::default().fg(Color::Gray);
    let light_red = Style::default().fg(Color::LightRed);
    let red = Style::default().fg(Color::Red);
    let green = Style::default().fg(Color::Green);
    let light_green = Style::default().fg(Color::LightGreen);

    match &app.message {
        Message::None => Paragraph::new("").style(dark_gray).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Messages")
                .border_style(dark_gray),
        ),

        Message::Error(error) => Paragraph::new(error.to_string()).style(red).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Error")
                .border_style(light_red.add_modifier(Modifier::BOLD)),
        ),

        Message::Success(string) => Paragraph::new(string.as_str()).style(green).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Success")
                .border_style(light_green.add_modifier(Modifier::BOLD)),
        ),

        Message::Debug(string) => Paragraph::new(string.as_str()).style(gray).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Debug")
                .border_style(dark_gray),
        ),
    }
}
