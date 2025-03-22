use crate::mancala::{GameState, Outcome};
use crate::packed_actions::Action;
use crate::player::{AIPlayer, Player};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Table, TableState,
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::time::Duration;

/// Struct to track game history for visualization
pub struct GameHistory {
    states: Vec<GameState>,
    values: Vec<f64>,
    actions: Vec<Action>,
}

impl GameHistory {
    fn new(initial_state: GameState, initial_value: f64) -> Self {
        GameHistory {
            states: vec![initial_state],
            values: vec![initial_value],
            actions: vec![],
        }
    }

    fn add_move(&mut self, state: GameState, value: f64, action: Action) {
        self.states.push(state);
        self.values.push(value);
        self.actions.push(action);
    }

    fn get_data_points(&self) -> Vec<(f64, f64)> {
        self.values
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v))
            .collect()
    }
}

/// App state
pub struct App<'a> {
    game_state: GameState,
    value_fn: &'a HashMap<GameState, f64>,
    move_table_state: TableState,
    possible_moves: Vec<(Action, GameState, f64)>,
    history: GameHistory,
    should_quit: bool,
    ai_player: AIPlayer,
    is_human_turn: bool,
    status_message: String,
}

impl<'a> App<'a> {
    pub fn new(initial_state: GameState, value_fn: &'a HashMap<GameState, f64>) -> Self {
        let initial_value = *value_fn.get(&initial_state).unwrap_or(&0.5);
        
        // Create AI player with opponent's perspective
        let mut ai_starting_state = initial_state.clone();
        ai_starting_state.swap_board();
        let ai_player = AIPlayer::new(ai_starting_state);
        
        let mut app = App {
            game_state: initial_state.clone(),
            value_fn,
            move_table_state: TableState::default(),
            possible_moves: Vec::new(),
            history: GameHistory::new(initial_state, initial_value),
            should_quit: false,
            ai_player,
            is_human_turn: true,
            status_message: String::from("Your turn. Select a move."),
        };
        app.update_possible_moves();
        app.move_table_state.select(Some(0));
        app
    }
    
    pub fn reset_game(&mut self) {
        // Create a new game state
        let initial_state = GameState::new(4);
        let initial_value = *self.value_fn.get(&initial_state).unwrap_or(&0.5);
        
        // Create AI player with opponent's perspective
        let mut ai_starting_state = initial_state.clone();
        ai_starting_state.swap_board();
        
        // Reset app state
        self.game_state = initial_state.clone();
        self.move_table_state = TableState::default();
        self.possible_moves = Vec::new();
        self.history = GameHistory::new(initial_state, initial_value);
        self.ai_player = AIPlayer::new(ai_starting_state);
        self.is_human_turn = true;
        self.status_message = String::from("New game started. Your turn!");
        
        // Update moves
        self.update_possible_moves();
        self.move_table_state.select(Some(0));
    }
    
    pub fn ai_turn(&mut self) {
        if !self.is_human_turn && !self.is_game_over() {
            self.status_message = String::from("AI is thinking...");
            
            // Let AI make a move
            let action = self.ai_player.take_action(self.value_fn, 0.0);
            
            // Update our game state with the AI's move
            self.game_state.swap_board();
            self.game_state.evaluate_action(action);
            self.game_state.swap_board();
            
            // Update history
            let value = *self.value_fn.get(&self.game_state).unwrap_or(&0.5);
            self.history.add_move(self.game_state.clone(), value, action);
            
            // Check if game is over after AI move
            if self.is_game_over() {
                self.handle_game_end();
                return;
            }
            
            // Update possible moves for human
            self.update_possible_moves();
            self.move_table_state.select(Some(0));
            
            self.is_human_turn = true;
            self.status_message = format!("AI played {}. Your turn now.", action);
        }
    }

    pub fn update_possible_moves(&mut self) {
        self.possible_moves = self
            .game_state
            .gen_actions()
            .map(|action| {
                let mut state = self.game_state;
                state.evaluate_action(action);
                let value = *self.value_fn.get(&state).unwrap_or(&0.5);
                (action, state, value)
            })
            .collect();

        // Sort by value, best moves first
        self.possible_moves.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    }

    pub fn next(&mut self) {
        let i = match self.move_table_state.selected() {
            Some(i) => {
                if i >= self.possible_moves.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.move_table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.move_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.possible_moves.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.move_table_state.select(Some(i));
    }

    pub fn make_selected_move(&mut self) {
        if self.is_human_turn && !self.is_game_over() {
            if let Some(selected) = self.move_table_state.selected() {
                if selected < self.possible_moves.len() {
                    let (action, new_state, value) = self.possible_moves[selected].clone();
                    self.game_state = new_state;
                    self.history.add_move(self.game_state.clone(), value, action);
                    
                    // Update AI's state with human's move
                    self.ai_player.opponent_plays(action);
                    
                    // Check if game is over after human move
                    if self.is_game_over() {
                        self.handle_game_end();
                        return;
                    }
                    
                    // Switch to AI's turn
                    self.is_human_turn = false;
                    self.status_message = String::from("AI's turn...");
                }
            }
        }
    }
    
    fn handle_game_end(&mut self) {
        use crate::mancala::Outcome::*;
        
        // Call finalize_game to move remaining stones to the appropriate store
        // This needs to happen before we calculate scores
        self.game_state.finalize_game();
        
        // Calculate final scores for display
        let player_score = self.game_state.houses[6];
        let ai_score = self.game_state.houses[13];
        
        match self.get_game_outcome() {
            Some(P1win) => self.status_message = format!(
                "GAME OVER! You WON! Score: {}-{} (Press 'r' to play again)",
                player_score, ai_score
            ),
            Some(P2win) => self.status_message = format!(
                "GAME OVER! AI WON! Score: {}-{} (Press 'r' to play again)",
                player_score, ai_score
            ),
            Some(Tie) => self.status_message = format!(
                "GAME OVER! It's a tie! Score: {}-{} (Press 'r' to play again)",
                player_score, ai_score
            ),
            _ => self.status_message = String::from("Game somehow ended without a result... (Press 'r' to play again)"),
        }
    }
    
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
    
    pub fn is_game_over(&self) -> bool {
        self.game_state.is_ended()
    }
    
    pub fn get_game_outcome(&self) -> Option<Outcome> {
        self.game_state.is_won()
    }
}

/// UI rendering
pub fn draw(f: &mut Frame, app: &App) {
    // Create the layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Game board
            Constraint::Length(3),  // Status message
            Constraint::Min(10),    // Move analysis
            Constraint::Length(9),  // Win probability chart
            Constraint::Length(3),  // Controls
        ])
        .split(f.size());

    // Draw the game board
    draw_game_board(f, app, chunks[0]);
    
    // Draw status message
    draw_status_message(f, app, chunks[1]);
    
    // Draw the move analysis table
    draw_move_analysis(f, app, chunks[2]);
    
    // Draw the win probability chart
    draw_win_probability(f, app, chunks[3]);
    
    // Draw controls
    draw_controls(f, app, chunks[4]);
}

fn draw_game_board(f: &mut Frame, app: &App, area: Rect) {
    // Create a container block for the entire board
    let board_block = Block::default()
        .borders(Borders::ALL)
        .title("Game Board");
    
    // Get the inner area to work with (inside borders)
    let inner_area = board_block.inner(area);
    
    // Render the container
    f.render_widget(board_block, area);
    
    // Define styles
    let cell_style = Style::default().bg(Color::Black);
    let cell_number_style = Style::default().fg(Color::DarkGray);
    let p1_cells_style = Style::default().fg(Color::Green);
    let p2_cells_style = Style::default().fg(Color::Yellow);
    let mancala_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    
    // First, create main horizontal layout with left mancala, center board, right mancala
    let main_horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 8),  // P2 Mancala (left)
            Constraint::Ratio(6, 8),  // Center board
            Constraint::Ratio(1, 8),  // P1 Mancala (right)
        ])
        .split(inner_area);
    
    // Define vertical layout for the center board (2 rows)
    let center_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 2),  // Top row (P2)
            Constraint::Ratio(1, 2),  // Bottom row (P1)
        ])
        .split(main_horizontal[1]);
    
    // Define columns for top row (P2) - cells 12 to 7
    let p2_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .split(center_vertical[0]);
    
    // Define columns for bottom row (P1) - cells 1 to 6
    let p1_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .split(center_vertical[1]);
    
    // Render P2's cells (cells 12 to 7, reversed)
    for (i, rect) in p2_columns.iter().enumerate() {
        // Cell index (12 to 7, reversed)
        let cell_idx = 12 - i;
        let house_idx = cell_idx as usize;
        
        // Stone count
        let stone_count = app.game_state.houses[house_idx];
        
        // Create cell with block and cell number as title
        let cell_block = Block::default()
            .borders(Borders::ALL)
            .style(cell_style)
            .title(format!("{}", cell_idx));
        
        f.render_widget(cell_block.clone(), *rect);
        
        // Get inner area of the cell
        let inner_cell = cell_block.inner(*rect);
        
        // Add stone count centered in the cell
        let stone_text = Paragraph::new(stone_count.to_string())
            .style(p2_cells_style)
            .alignment(ratatui::layout::Alignment::Center);
        
        // Center the stone count in the inner cell
        let stone_rect = Rect::new(
            inner_cell.x,
            inner_cell.y + inner_cell.height / 2,  // Center vertically
            inner_cell.width,
            1
        );
        
        f.render_widget(stone_text, stone_rect);
    }
    
    // Render P1's cells (cells 1 to 6)
    for (i, rect) in p1_columns.iter().enumerate() {
        // Cell index (1 to 6)
        let cell_idx = i + 1;
        let house_idx = i;
        
        // Stone count
        let stone_count = app.game_state.houses[house_idx];
        
        // Create cell with cell number as title
        let cell_block = Block::default()
            .borders(Borders::ALL)
            .style(cell_style)
            .title(format!("{}", cell_idx));
        
        f.render_widget(cell_block.clone(), *rect);
        
        // Get inner area of the cell
        let inner_cell = cell_block.inner(*rect);
        
        // Add stone count centered in the cell
        let stone_text = Paragraph::new(stone_count.to_string())
            .style(p1_cells_style)
            .alignment(ratatui::layout::Alignment::Center);
        
        // Center the stone count in the inner cell
        let stone_rect = Rect::new(
            inner_cell.x,
            inner_cell.y + inner_cell.height / 2,  // Center vertically
            inner_cell.width,
            1
        );
        
        f.render_widget(stone_text, stone_rect);
    }
    
    // Render P2's Mancala (left side)
    let p2_mancala_block = Block::default()
        .borders(Borders::ALL)
        .style(cell_style)
        .title("13");
    
    f.render_widget(p2_mancala_block.clone(), main_horizontal[0]);
    
    // Get inner area of the mancala
    let inner_p2_mancala = p2_mancala_block.inner(main_horizontal[0]);
    
    // Add "P2" label at the top of the mancala
    let p2_label = Paragraph::new("P2")
        .style(cell_number_style)
        .alignment(ratatui::layout::Alignment::Center);
    
    let p2_label_rect = Rect::new(
        inner_p2_mancala.x,
        inner_p2_mancala.y + 1,  // Near the top
        inner_p2_mancala.width,
        1
    );
    
    f.render_widget(p2_label, p2_label_rect);
    
    // Add stone count to P2's Mancala
    let p2_mancala_text = Paragraph::new(app.game_state.houses[13].to_string())
        .style(mancala_style)
        .alignment(ratatui::layout::Alignment::Center);
    
    // Center the stone count vertically in the mancala
    let p2_mancala_rect = Rect::new(
        inner_p2_mancala.x,
        inner_p2_mancala.y + inner_p2_mancala.height / 2,  // Center vertically
        inner_p2_mancala.width,
        1
    );
    
    f.render_widget(p2_mancala_text, p2_mancala_rect);
    
    // Render P1's Mancala (right side)
    let p1_mancala_block = Block::default()
        .borders(Borders::ALL)
        .style(cell_style)
        .title("6");
    
    f.render_widget(p1_mancala_block.clone(), main_horizontal[2]);
    
    // Get inner area of the mancala
    let inner_p1_mancala = p1_mancala_block.inner(main_horizontal[2]);
    
    // Add "P1" label at the top of the mancala
    let p1_label = Paragraph::new("P1")
        .style(cell_number_style)
        .alignment(ratatui::layout::Alignment::Center);
    
    let p1_label_rect = Rect::new(
        inner_p1_mancala.x,
        inner_p1_mancala.y + 1,  // Near the top
        inner_p1_mancala.width,
        1
    );
    
    f.render_widget(p1_label, p1_label_rect);
    
    // Add stone count to P1's Mancala
    let p1_mancala_text = Paragraph::new(app.game_state.houses[6].to_string())
        .style(mancala_style)
        .alignment(ratatui::layout::Alignment::Center);
    
    // Center the stone count vertically in the mancala
    let p1_mancala_rect = Rect::new(
        inner_p1_mancala.x,
        inner_p1_mancala.y + inner_p1_mancala.height / 2,  // Center vertically
        inner_p1_mancala.width,
        1
    );
    
    f.render_widget(p1_mancala_text, p1_mancala_rect);
}

fn draw_move_analysis(f: &mut Frame, app: &App, area: Rect) {
    let selected_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    
    let mut rows = Vec::new();
    let best_value = if app.possible_moves.is_empty() {
        0.5
    } else {
        app.possible_moves[0].2
    };
    
    for (i, (action, _, value)) in app.possible_moves.iter().enumerate() {
        let diff = value - best_value;
        let move_style = if i == 0 {
            Style::default().fg(Color::Green)
        } else if i == app.possible_moves.len() - 1 {
            Style::default().fg(Color::Red)
        } else if diff > -0.1 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        // Create a selection indicator in its own column
        let selection_indicator = if app.move_table_state.selected().map_or(false, |selected| selected == i) {
            "▶"
        } else {
            " "
        };
        
        // Create separate cells for each column
        let cells = vec![
            Span::styled(selection_indicator, Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}", action), move_style),
            Span::styled(format!("{:.6}", value), move_style),
            Span::styled(format!("{:+.6}", diff), move_style),
            Span::styled(format!("{}", i + 1), Style::default()),
        ];
        
        // Create a row with individual cells for proper column separation
        rows.push(Row::new(cells));
    }
    
    // Create header cells with matching spans
    let header_cells = vec![
        Span::styled(" ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("Move", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("Value", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("Diff", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("#", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ];
    
    let header = Row::new(header_cells);
    
    let widths = [
        Constraint::Length(2),         // Selection indicator
        Constraint::Length(20),        // Move - expanded to show multi-turn actions better
        Constraint::Length(10),        // Expected Value (fixed width)
        Constraint::Length(10),        // Diff from Best (fixed width)
        Constraint::Length(3),         // Position
    ];
    
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Possible Moves"))
        .highlight_style(selected_style);
    
    f.render_stateful_widget(table, area, &mut app.move_table_state.clone());
}

fn draw_win_probability(f: &mut Frame, app: &App, area: Rect) {
    let data = app.history.get_data_points();
    if data.is_empty() {
        return;
    }
    
    // Create data for the chart
    let dataset = Dataset::default()
        .name("Win Probability")
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data);
    
    let x_labels = vec![
        Span::styled("0", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}", data.len() - 1),
            Style::default().fg(Color::White),
        ),
    ];
    
    let y_labels = vec![
        Span::styled("0.0", Style::default().fg(Color::White)),
        Span::styled("0.5", Style::default().fg(Color::White)),
        Span::styled("1.0", Style::default().fg(Color::White)),
    ];
    
    let chart = Chart::new(vec![dataset])
        .block(Block::default().title("Win Probability").borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .title("Turn")
                .bounds([0.0, (data.len() - 1) as f64])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("Probability")
                .bounds([0.0, 1.0])
                .labels(y_labels),
        );
    
    f.render_widget(chart, area);
}

fn draw_status_message(f: &mut Frame, app: &App, area: Rect) {
    let status_style = if app.is_game_over() {
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
    } else if app.is_human_turn {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };
    
    let status = Span::styled(&app.status_message, status_style);
    let paragraph = Paragraph::new(Line::from(vec![status]))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    
    f.render_widget(paragraph, area);
}

fn draw_controls(f: &mut Frame, app: &App, area: Rect) {
    let mut controls = vec![
        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Select Move | "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Play Move | "),
    ];
    
    // Add restart control if game is over
    if app.is_game_over() {
        controls.push(Span::styled("r", Style::default().fg(Color::Green)));
        controls.push(Span::raw(" Restart | "));
    }
    
    controls.push(Span::styled("q", Style::default().fg(Color::Yellow)));
    controls.push(Span::raw(" Quit"));
    
    let paragraph = Paragraph::new(Line::from(controls))
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    
    f.render_widget(paragraph, area);
}

/// Terminal setup and handling
pub fn run_tui(
    starting_state: GameState,
    value_fun: &HashMap<GameState, f64>,
) -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(starting_state, value_fun);
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if app.is_game_over() {
            // When game is over, display the result but keep the UI available
            // to review the final state until user quits or resets
            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('r') => {
                            app.reset_game();
                            continue;
                        },
                        _ => {}
                    }
                }
            }
            continue;
        }

        // Check if it's AI's turn and trigger AI move
        if !app.is_human_turn && !app.is_game_over() {
            app.ai_turn();
            terminal.draw(|f| draw(f, app))?;
            continue;
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.quit();
                        return Ok(());
                    }
                    KeyCode::Char('r') => app.reset_game(),
                    KeyCode::Up => app.previous(),
                    KeyCode::Down => app.next(),
                    KeyCode::Enter => app.make_selected_move(),
                    _ => {}
                }
            }
        }
        
        if app.should_quit {
            return Ok(());
        }
    }
}
