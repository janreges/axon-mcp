# SERVER02: Create Agent CLI Application

## Objective
Create a command-line interface application for agents to interact with the MCP server, supporting all agent operations including task discovery, messaging, knowledge management, and collaboration features.

## Implementation Details

### 1. Create Main CLI Application
In `mcp-server/src/bin/mcp-agent.rs`:

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use mcp_client::{McpClient, AgentSession};
use std::path::PathBuf;
use tokio::io::{self, AsyncBufReadExt};
use tracing::{info, error, warn};
use colored::*;

mod commands;
mod interactive;
mod display;
mod config;

use crate::commands::*;
use crate::interactive::InteractiveMode;
use crate::config::AgentConfig;

#[derive(Parser, Debug)]
#[command(author, version, about = "MCP Agent CLI - Interact with MCP task management system")]
struct Cli {
    /// MCP server URL
    #[arg(short, long, env = "MCP_SERVER_URL", default_value = "http://localhost:8080")]
    server: String,
    
    /// Agent name (required for most commands)
    #[arg(short, long, env = "MCP_AGENT_NAME")]
    agent: Option<String>,
    
    /// Configuration file
    #[arg(short, long, default_value = "~/.mcp-agent/config.toml")]
    config: PathBuf,
    
    /// Output format
    #[arg(short = 'f', long, default_value = "table")]
    format: OutputFormat,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Table,
    Json,
    Yaml,
    Plain,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            "plain" => Ok(OutputFormat::Plain),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Register as an agent
    Register {
        /// Agent capabilities (can be specified multiple times)
        #[arg(short, long)]
        capability: Vec<String>,
        
        /// Agent specializations
        #[arg(short, long)]
        specialization: Vec<String>,
        
        /// Maximum concurrent tasks
        #[arg(short, long, default_value = "5")]
        max_tasks: i32,
        
        /// Agent description
        #[arg(short, long)]
        description: Option<String>,
    },
    
    /// Start agent and begin work discovery
    Start {
        /// Enable long-polling for work discovery
        #[arg(short, long)]
        poll: bool,
        
        /// Auto-accept suitable tasks
        #[arg(short, long)]
        auto_accept: bool,
    },
    
    /// Discover available work
    Discover {
        /// Maximum number of tasks to return
        #[arg(short, long, default_value = "10")]
        limit: i32,
        
        /// Filter by capability
        #[arg(short, long)]
        capability: Option<String>,
    },
    
    /// Task operations
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },
    
    /// Message operations
    Message {
        #[command(subcommand)]
        command: MessageCommands,
    },
    
    /// Knowledge base operations
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommands,
    },
    
    /// Help request operations
    Help {
        #[command(subcommand)]
        command: HelpCommands,
    },
    
    /// Handoff operations
    Handoff {
        #[command(subcommand)]
        command: HandoffCommands,
    },
    
    /// View agent metrics and stats
    Stats {
        /// Time period (day, week, month)
        #[arg(short, long, default_value = "week")]
        period: String,
    },
    
    /// Interactive mode
    Interactive,
    
    /// Send heartbeat
    Heartbeat {
        /// Current task load
        #[arg(short, long)]
        load: Option<i32>,
        
        /// New status
        #[arg(short, long)]
        status: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum TaskCommands {
    /// List assigned tasks
    List {
        /// Filter by state
        #[arg(short, long)]
        state: Option<String>,
        
        /// Include completed tasks
        #[arg(long)]
        all: bool,
    },
    
    /// Accept a task
    Accept {
        /// Task code
        code: String,
    },
    
    /// Update task progress
    Progress {
        /// Task code
        code: String,
        
        /// Progress percentage (0-100)
        #[arg(short, long)]
        percent: Option<u8>,
        
        /// Status message
        #[arg(short, long)]
        message: Option<String>,
    },
    
    /// Complete a task
    Complete {
        /// Task code
        code: String,
        
        /// Completion notes
        #[arg(short, long)]
        notes: Option<String>,
    },
    
    /// Block a task
    Block {
        /// Task code
        code: String,
        
        /// Reason for blocking
        reason: String,
    },
    
    /// Decompose task into subtasks
    Decompose {
        /// Parent task code
        code: String,
        
        /// Subtask definitions (JSON)
        subtasks: String,
    },
}

#[derive(Subcommand, Debug)]
enum MessageCommands {
    /// Send a message
    Send {
        /// Task code
        #[arg(short, long)]
        task: String,
        
        /// Message type
        #[arg(short = 't', long)]
        msg_type: String,
        
        /// Message content
        message: String,
        
        /// Reply to message ID
        #[arg(short, long)]
        reply_to: Option<i32>,
    },
    
    /// List messages
    List {
        /// Task code
        #[arg(short, long)]
        task: String,
        
        /// Message type filter
        #[arg(short = 't', long)]
        msg_type: Option<String>,
        
        /// Show only unread
        #[arg(short, long)]
        unread: bool,
    },
    
    /// Search messages
    Search {
        /// Search query
        query: String,
        
        /// Task codes to search
        #[arg(short, long)]
        task: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum KnowledgeCommands {
    /// Create knowledge object
    Create {
        /// Task code
        #[arg(short, long)]
        task: String,
        
        /// Knowledge type
        #[arg(short = 't', long)]
        knowledge_type: String,
        
        /// Title
        title: String,
        
        /// Body content or file path
        body: String,
        
        /// Tags
        #[arg(long)]
        tag: Vec<String>,
        
        /// Visibility
        #[arg(short, long, default_value = "team")]
        visibility: String,
    },
    
    /// Search knowledge base
    Search {
        /// Search query
        query: String,
        
        /// Filter by task
        #[arg(short, long)]
        task: Option<String>,
        
        /// Filter by type
        #[arg(short = 't', long)]
        knowledge_type: Option<String>,
    },
    
    /// Export knowledge
    Export {
        /// Task code
        task: String,
        
        /// Output format (markdown, json)
        #[arg(short, long, default_value = "markdown")]
        format: String,
        
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum HelpCommands {
    /// Request help
    Request {
        /// Task code
        #[arg(short, long)]
        task: String,
        
        /// Help type
        #[arg(short = 't', long)]
        help_type: String,
        
        /// Description
        description: String,
        
        /// Urgency
        #[arg(short, long, default_value = "medium")]
        urgency: String,
        
        /// Required capabilities
        #[arg(short, long)]
        capability: Vec<String>,
    },
    
    /// List help requests
    List {
        /// Show only my requests
        #[arg(short, long)]
        mine: bool,
        
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
        
        /// Filter by capability
        #[arg(short, long)]
        capability: Option<String>,
    },
    
    /// Claim a help request
    Claim {
        /// Help request ID
        id: i32,
    },
    
    /// Resolve a help request
    Resolve {
        /// Help request ID
        id: i32,
        
        /// Resolution description
        resolution: String,
    },
}

#[derive(Subcommand, Debug)]
enum HandoffCommands {
    /// Create handoff package
    Create {
        /// Task code
        task: String,
        
        /// Target agent (optional)
        #[arg(short, long)]
        to: Option<String>,
        
        /// Target capability (if no specific agent)
        #[arg(short, long)]
        capability: Option<String>,
        
        /// Handoff context
        context: String,
        
        /// Recommended next steps
        #[arg(short, long)]
        next_steps: Option<String>,
    },
    
    /// List available handoffs
    List {
        /// Show only handoffs for me
        #[arg(short, long)]
        mine: bool,
        
        /// Include accepted/rejected
        #[arg(short, long)]
        all: bool,
    },
    
    /// Accept a handoff
    Accept {
        /// Handoff ID
        id: i32,
        
        /// Import knowledge snapshot
        #[arg(long, default_value = "true")]
        import_knowledge: bool,
    },
    
    /// Reject a handoff
    Reject {
        /// Handoff ID
        id: i32,
        
        /// Rejection reason
        reason: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(cli.verbose);
    
    // Load configuration
    let config = AgentConfig::load(&cli.config)?;
    
    // Determine agent name
    let agent_name = cli.agent
        .or(config.default_agent.clone())
        .ok_or_else(|| anyhow::anyhow!("Agent name required. Use --agent or set default in config"))?;
    
    // Create MCP client
    let client = McpClient::new(&cli.server)?;
    
    // Handle commands
    match cli.command {
        None => {
            // No command specified, show help
            println!("{}", "MCP Agent CLI".bold());
            println!("Use --help for usage information");
            println!("\nQuick start:");
            println!("  {} - Start in interactive mode", "mcp-agent interactive".green());
            println!("  {} - Discover available work", "mcp-agent discover".green());
            println!("  {} - View your tasks", "mcp-agent task list".green());
        }
        
        Some(Commands::Interactive) => {
            let session = AgentSession::new(client, agent_name).await?;
            let mut interactive = InteractiveMode::new(session);
            interactive.run().await?;
        }
        
        Some(Commands::Register { capability, specialization, max_tasks, description }) => {
            commands::register_agent(
                &client,
                &agent_name,
                capability,
                specialization,
                max_tasks,
                description,
            ).await?;
        }
        
        Some(Commands::Start { poll, auto_accept }) => {
            commands::start_agent(&client, &agent_name, poll, auto_accept).await?;
        }
        
        Some(Commands::Discover { limit, capability }) => {
            let tasks = commands::discover_work(&client, &agent_name, limit, capability).await?;
            display::show_tasks(&tasks, cli.format);
        }
        
        Some(Commands::Task { command }) => {
            handle_task_command(&client, &agent_name, command, cli.format).await?;
        }
        
        Some(Commands::Message { command }) => {
            handle_message_command(&client, &agent_name, command, cli.format).await?;
        }
        
        Some(Commands::Knowledge { command }) => {
            handle_knowledge_command(&client, &agent_name, command, cli.format).await?;
        }
        
        Some(Commands::Help { command }) => {
            handle_help_command(&client, &agent_name, command, cli.format).await?;
        }
        
        Some(Commands::Handoff { command }) => {
            handle_handoff_command(&client, &agent_name, command, cli.format).await?;
        }
        
        Some(Commands::Stats { period }) => {
            let stats = commands::get_agent_stats(&client, &agent_name, &period).await?;
            display::show_agent_stats(&stats, cli.format);
        }
        
        Some(Commands::Heartbeat { load, status }) => {
            commands::send_heartbeat(&client, &agent_name, load, status).await?;
            println!("✓ Heartbeat sent");
        }
    }
    
    Ok(())
}

fn init_logging(verbose: bool) {
    use tracing_subscriber::fmt::format::FmtSpan;
    
    let filter = if verbose {
        "debug,hyper=info"
    } else {
        "warn,mcp_agent=info"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .with_target(false)
        .init();
}

async fn handle_task_command(
    client: &McpClient,
    agent_name: &str,
    command: TaskCommands,
    format: OutputFormat,
) -> Result<()> {
    match command {
        TaskCommands::List { state, all } => {
            let tasks = commands::list_tasks(client, agent_name, state, all).await?;
            display::show_tasks(&tasks, format);
        }
        
        TaskCommands::Accept { code } => {
            commands::accept_task(client, agent_name, &code).await?;
            println!("✓ Task {} accepted", code.green());
        }
        
        TaskCommands::Progress { code, percent, message } => {
            commands::update_progress(client, agent_name, &code, percent, message).await?;
            println!("✓ Progress updated for task {}", code.green());
        }
        
        TaskCommands::Complete { code, notes } => {
            commands::complete_task(client, agent_name, &code, notes).await?;
            println!("✓ Task {} completed", code.green());
        }
        
        TaskCommands::Block { code, reason } => {
            commands::block_task(client, agent_name, &code, &reason).await?;
            println!("✓ Task {} blocked", code.yellow());
        }
        
        TaskCommands::Decompose { code, subtasks } => {
            let count = commands::decompose_task(client, agent_name, &code, &subtasks).await?;
            println!("✓ Task {} decomposed into {} subtasks", code.green(), count);
        }
    }
    
    Ok(())
}

// Additional command handlers would follow similar pattern...
```

### 2. Create Interactive Mode
In `mcp-server/src/bin/mcp-agent/interactive.rs`:

```rust
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;
use tokio::sync::mpsc;

pub struct InteractiveMode {
    session: AgentSession,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    current_tab: usize,
    task_list: Vec<Task>,
    message_list: Vec<TaskMessage>,
    selected_task: Option<String>,
    input_buffer: String,
    mode: Mode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

impl InteractiveMode {
    pub fn new(session: AgentSession) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        
        Ok(Self {
            session,
            terminal,
            current_tab: 0,
            task_list: Vec::new(),
            message_list: Vec::new(),
            selected_task: None,
            input_buffer: String::new(),
            mode: Mode::Normal,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        // Start background refresh
        let (refresh_tx, mut refresh_rx) = mpsc::channel(10);
        self.start_refresh_task(refresh_tx);
        
        loop {
            // Draw UI
            self.terminal.draw(|f| self.ui(f))?;
            
            // Handle events
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.handle_key(key).await? {
                        break;
                    }
                }
            }
            
            // Handle refresh events
            if let Ok(_) = refresh_rx.try_recv() {
                self.refresh_data().await?;
            }
        }
        
        // Cleanup
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        
        Ok(())
    }
    
    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // Main content
                Constraint::Length(3),  // Status bar
            ])
            .split(f.size());
        
        // Header with tabs
        let tab_titles = vec!["Tasks", "Messages", "Knowledge", "Help", "Metrics"];
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("MCP Agent"))
            .select(self.current_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        
        f.render_widget(tabs, chunks[0]);
        
        // Main content area
        match self.current_tab {
            0 => self.render_tasks(f, chunks[1]),
            1 => self.render_messages(f, chunks[1]),
            2 => self.render_knowledge(f, chunks[1]),
            3 => self.render_help(f, chunks[1]),
            4 => self.render_metrics(f, chunks[1]),
            _ => {}
        }
        
        // Status bar
        self.render_status_bar(f, chunks[2]);
    }
    
    fn render_tasks(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        
        // Task list
        let tasks: Vec<ListItem> = self.task_list.iter()
            .map(|task| {
                let style = match task.state {
                    TaskState::InProgress => Style::default().fg(Color::Yellow),
                    TaskState::Blocked => Style::default().fg(Color::Red),
                    TaskState::Review => Style::default().fg(Color::Cyan),
                    TaskState::Done => Style::default().fg(Color::Green),
                    _ => Style::default(),
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(&task.code, style.add_modifier(Modifier::BOLD)),
                        Span::raw(" - "),
                        Span::raw(&task.name),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("[{}]", task.state),
                            style,
                        ),
                        Span::raw(format!(" Priority: {}", task.priority_score)),
                    ]),
                ])
            })
            .collect();
        
        let tasks_list = List::new(tasks)
            .block(Block::default().borders(Borders::ALL).title("Tasks"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ");
        
        f.render_widget(tasks_list, chunks[0]);
        
        // Task details
        if let Some(selected_code) = &self.selected_task {
            if let Some(task) = self.task_list.iter().find(|t| &t.code == selected_code) {
                let details = vec![
                    Line::from(vec![
                        Span::styled("Code: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&task.code),
                    ]),
                    Line::from(vec![
                        Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&task.name),
                    ]),
                    Line::from(vec![
                        Span::styled("State: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(task.state.to_string()),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Description:", Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(&task.description),
                ];
                
                let details_widget = Paragraph::new(details)
                    .block(Block::default().borders(Borders::ALL).title("Task Details"))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                
                f.render_widget(details_widget, chunks[1]);
            }
        }
    }
    
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let mode_str = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
        };
        
        let status = if self.mode == Mode::Command {
            format!(":{}", self.input_buffer)
        } else {
            format!("[{}] {} | Tasks: {} | Connected", 
                mode_str,
                self.session.agent_name(),
                self.task_list.len()
            )
        };
        
        let status_bar = Paragraph::new(status)
            .style(match self.mode {
                Mode::Normal => Style::default().bg(Color::Blue).fg(Color::White),
                Mode::Insert => Style::default().bg(Color::Green).fg(Color::Black),
                Mode::Command => Style::default().bg(Color::Yellow).fg(Color::Black),
            })
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(status_bar, area);
    }
    
    async fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        match self.mode {
            Mode::Normal => match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Tab => {
                    self.current_tab = (self.current_tab + 1) % 5;
                }
                KeyCode::BackTab => {
                    self.current_tab = (self.current_tab + 4) % 5;
                }
                KeyCode::Char(':') => {
                    self.mode = Mode::Command;
                    self.input_buffer.clear();
                }
                KeyCode::Char('r') => {
                    self.refresh_data().await?;
                }
                KeyCode::Char('a') => {
                    if let Some(task_code) = &self.selected_task {
                        self.session.accept_task(task_code).await?;
                        self.refresh_data().await?;
                    }
                }
                KeyCode::Char('c') => {
                    if let Some(task_code) = &self.selected_task {
                        self.session.complete_task(task_code, None).await?;
                        self.refresh_data().await?;
                    }
                }
                KeyCode::Up => {
                    // Move selection up
                }
                KeyCode::Down => {
                    // Move selection down
                }
                KeyCode::Enter => {
                    // Select current item
                }
                _ => {}
            },
            
            Mode::Command => match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    self.input_buffer.clear();
                }
                KeyCode::Enter => {
                    self.execute_command().await?;
                    self.mode = Mode::Normal;
                    self.input_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            },
            
            Mode::Insert => {
                // Handle insert mode
            }
        }
        
        Ok(false)
    }
    
    async fn execute_command(&mut self) -> Result<()> {
        let parts: Vec<&str> = self.input_buffer.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        
        match parts[0] {
            "refresh" | "r" => self.refresh_data().await?,
            "discover" | "d" => {
                let tasks = self.session.discover_work(10).await?;
                // Show tasks in popup
            }
            "help" | "h" => {
                // Show help
            }
            "quit" | "q" => {
                // Quit (handled in normal mode)
            }
            _ => {
                // Unknown command
            }
        }
        
        Ok(())
    }
    
    async fn refresh_data(&mut self) -> Result<()> {
        // Refresh task list
        self.task_list = self.session.list_tasks(None, false).await?;
        
        // Refresh messages if on messages tab
        if self.current_tab == 1 {
            if let Some(task_code) = &self.selected_task {
                self.message_list = self.session.get_messages(task_code).await?;
            }
        }
        
        Ok(())
    }
    
    fn start_refresh_task(&self, tx: mpsc::Sender<()>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                let _ = tx.send(()).await;
            }
        });
    }
}
```

### 3. Create Display Module
In `mcp-server/src/bin/mcp-agent/display.rs`:

```rust
use colored::*;
use prettytable::{Cell, Row, Table};
use serde::Serialize;

pub fn show_tasks(tasks: &[Task], format: OutputFormat) {
    match format {
        OutputFormat::Table => show_tasks_table(tasks),
        OutputFormat::Json => show_json(tasks),
        OutputFormat::Yaml => show_yaml(tasks),
        OutputFormat::Plain => show_tasks_plain(tasks),
    }
}

fn show_tasks_table(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("No tasks found");
        return;
    }
    
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Code").style_spec("Fb"),
        Cell::new("Name").style_spec("Fb"),
        Cell::new("State").style_spec("Fb"),
        Cell::new("Priority").style_spec("Fb"),
        Cell::new("Owner").style_spec("Fb"),
        Cell::new("Created").style_spec("Fb"),
    ]));
    
    for task in tasks {
        let state_cell = match task.state {
            TaskState::Created => Cell::new(&task.state.to_string()).style_spec("Fw"),
            TaskState::InProgress => Cell::new(&task.state.to_string()).style_spec("Fy"),
            TaskState::Blocked => Cell::new(&task.state.to_string()).style_spec("Fr"),
            TaskState::Review => Cell::new(&task.state.to_string()).style_spec("Fc"),
            TaskState::Done => Cell::new(&task.state.to_string()).style_spec("Fg"),
            _ => Cell::new(&task.state.to_string()),
        };
        
        table.add_row(Row::new(vec![
            Cell::new(&task.code),
            Cell::new(&task.name),
            state_cell,
            Cell::new(&task.priority_score.to_string()),
            Cell::new(&task.owner_agent_name),
            Cell::new(&task.inserted_at.format("%Y-%m-%d %H:%M").to_string()),
        ]));
    }
    
    table.printstd();
}

fn show_tasks_plain(tasks: &[Task]) {
    for task in tasks {
        println!("{} - {} [{}] (Priority: {})",
            task.code.bold(),
            task.name,
            colored_state(&task.state),
            task.priority_score
        );
    }
}

fn colored_state(state: &TaskState) -> ColoredString {
    match state {
        TaskState::Created => state.to_string().white(),
        TaskState::InProgress => state.to_string().yellow(),
        TaskState::Blocked => state.to_string().red(),
        TaskState::Review => state.to_string().cyan(),
        TaskState::Done => state.to_string().green(),
        _ => state.to_string().normal(),
    }
}

pub fn show_messages(messages: &[TaskMessage], format: OutputFormat) {
    match format {
        OutputFormat::Table => show_messages_table(messages),
        OutputFormat::Json => show_json(messages),
        OutputFormat::Yaml => show_yaml(messages),
        OutputFormat::Plain => show_messages_plain(messages),
    }
}

fn show_messages_table(messages: &[TaskMessage]) {
    if messages.is_empty() {
        println!("No messages found");
        return;
    }
    
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("ID").style_spec("Fb"),
        Cell::new("Type").style_spec("Fb"),
        Cell::new("Author").style_spec("Fb"),
        Cell::new("Time").style_spec("Fb"),
        Cell::new("Content").style_spec("Fb"),
    ]));
    
    for msg in messages {
        let type_cell = match msg.message_type {
            MessageType::Question => Cell::new("Question").style_spec("Fy"),
            MessageType::Blocker => Cell::new("Blocker").style_spec("Fr"),
            MessageType::Solution => Cell::new("Solution").style_spec("Fg"),
            MessageType::Update => Cell::new("Update").style_spec("Fc"),
            _ => Cell::new(&msg.message_type.to_string()),
        };
        
        let content = if msg.content.len() > 50 {
            format!("{}...", &msg.content[..50])
        } else {
            msg.content.clone()
        };
        
        table.add_row(Row::new(vec![
            Cell::new(&msg.id.to_string()),
            type_cell,
            Cell::new(&msg.author_agent_name),
            Cell::new(&msg.created_at.format("%H:%M").to_string()),
            Cell::new(&content),
        ]));
    }
    
    table.printstd();
}

fn show_messages_plain(messages: &[TaskMessage]) {
    for msg in messages {
        println!("{} [{}] {}: {}",
            msg.created_at.format("%H:%M"),
            colored_message_type(&msg.message_type),
            msg.author_agent_name.blue(),
            msg.content
        );
        
        if let Some(reply_to) = msg.reply_to_message_id {
            println!("  {} Reply to #{}", "↳".dimmed(), reply_to);
        }
    }
}

fn colored_message_type(msg_type: &MessageType) -> ColoredString {
    match msg_type {
        MessageType::Comment => "Comment".normal(),
        MessageType::Question => "Question".yellow(),
        MessageType::Update => "Update".cyan(),
        MessageType::Blocker => "Blocker".red(),
        MessageType::Solution => "Solution".green(),
        MessageType::Review => "Review".blue(),
        _ => msg_type.to_string().normal(),
    }
}

pub fn show_agent_stats(stats: &AgentMetrics, format: OutputFormat) {
    match format {
        OutputFormat::Table => show_agent_stats_table(stats),
        OutputFormat::Json => show_json(stats),
        OutputFormat::Yaml => show_yaml(stats),
        OutputFormat::Plain => show_agent_stats_plain(stats),
    }
}

fn show_agent_stats_table(stats: &AgentMetrics) {
    println!("\n{}", format!("Agent Statistics: {}", stats.agent_name).bold());
    println!("{}", "─".repeat(50));
    
    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    
    table.add_row(Row::new(vec![
        Cell::new("Metric"),
        Cell::new("Value"),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Tasks Completed"),
        Cell::new(&stats.tasks_completed.to_string()).style_spec("Fg"),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Tasks In Progress"),
        Cell::new(&stats.tasks_in_progress.to_string()).style_spec("Fy"),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Success Rate"),
        Cell::new(&format!("{:.1}%", stats.success_rate * 100.0))
            .style_spec(if stats.success_rate > 0.9 { "Fg" } else { "Fy" }),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Avg Task Duration"),
        Cell::new(&format!("{} min", stats.average_task_duration_minutes)),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Reputation Score"),
        Cell::new(&format!("{:.2}", stats.reputation_score))
            .style_spec(if stats.reputation_score > 0.8 { "Fg" } else { "Fw" }),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Help Provided"),
        Cell::new(&stats.help_provided.to_string()),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Help Requested"),
        Cell::new(&stats.help_requested.to_string()),
    ]));
    
    table.printstd();
    
    // Show capability utilization
    if !stats.capability_utilization.is_empty() {
        println!("\n{}", "Capability Utilization:".bold());
        for cap in &stats.capability_utilization {
            let bar_length = (cap.utilization_percentage / 5.0) as usize;
            let bar = "█".repeat(bar_length);
            let empty = "░".repeat(20 - bar_length);
            
            println!("  {:15} {} {:.1}%",
                cap.capability,
                format!("{}{}", bar.green(), empty.dimmed()),
                cap.utilization_percentage
            );
        }
    }
}

fn show_agent_stats_plain(stats: &AgentMetrics) {
    println!("Agent: {}", stats.agent_name.bold());
    println!("Tasks: {} completed, {} in progress", 
        stats.tasks_completed.to_string().green(),
        stats.tasks_in_progress.to_string().yellow()
    );
    println!("Success Rate: {:.1}%", stats.success_rate * 100.0);
    println!("Reputation: {:.2}", stats.reputation_score);
}

fn show_json<T: Serialize>(data: &T) {
    match serde_json::to_string_pretty(data) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing to JSON: {}", e),
    }
}

fn show_yaml<T: Serialize>(data: &T) {
    match serde_yaml::to_string(data) {
        Ok(yaml) => println!("{}", yaml),
        Err(e) => eprintln!("Error serializing to YAML: {}", e),
    }
}
```

### 4. Create Shell Completions
In `mcp-server/completions/mcp-agent.bash`:

```bash
#!/bin/bash

_mcp_agent_completions() {
    local cur prev opts base
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Main commands
    local commands="register start discover task message knowledge help handoff stats interactive heartbeat"
    
    # Task subcommands
    local task_commands="list accept progress complete block decompose"
    
    # Message subcommands
    local message_commands="send list search"
    
    # Knowledge subcommands
    local knowledge_commands="create search export"
    
    # Help subcommands
    local help_commands="request list claim resolve"
    
    # Handoff subcommands
    local handoff_commands="create list accept reject"
    
    case "${COMP_CWORD}" in
        1)
            COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
            return 0
            ;;
        2)
            case "${prev}" in
                task)
                    COMPREPLY=( $(compgen -W "${task_commands}" -- ${cur}) )
                    return 0
                    ;;
                message)
                    COMPREPLY=( $(compgen -W "${message_commands}" -- ${cur}) )
                    return 0
                    ;;
                knowledge)
                    COMPREPLY=( $(compgen -W "${knowledge_commands}" -- ${cur}) )
                    return 0
                    ;;
                help)
                    COMPREPLY=( $(compgen -W "${help_commands}" -- ${cur}) )
                    return 0
                    ;;
                handoff)
                    COMPREPLY=( $(compgen -W "${handoff_commands}" -- ${cur}) )
                    return 0
                    ;;
            esac
            ;;
    esac
    
    # Handle options
    case "${prev}" in
        --format|-f)
            COMPREPLY=( $(compgen -W "table json yaml plain" -- ${cur}) )
            return 0
            ;;
        --state|-s)
            COMPREPLY=( $(compgen -W "created in_progress blocked review done" -- ${cur}) )
            return 0
            ;;
        --msg-type|-t)
            COMPREPLY=( $(compgen -W "comment question update blocker solution review" -- ${cur}) )
            return 0
            ;;
        --urgency|-u)
            COMPREPLY=( $(compgen -W "low medium high critical" -- ${cur}) )
            return 0
            ;;
    esac
    
    # Default to file completion for unhandled cases
    COMPREPLY=( $(compgen -f -- ${cur}) )
}

complete -F _mcp_agent_completions mcp-agent
```

## Files to Create
- `mcp-server/src/bin/mcp-agent.rs` - Main CLI application
- `mcp-server/src/bin/mcp-agent/commands.rs` - Command implementations
- `mcp-server/src/bin/mcp-agent/interactive.rs` - Interactive TUI mode
- `mcp-server/src/bin/mcp-agent/display.rs` - Output formatting
- `mcp-server/src/bin/mcp-agent/config.rs` - Configuration management
- `mcp-server/completions/mcp-agent.bash` - Shell completions
- `mcp-server/completions/mcp-agent.zsh` - ZSH completions
- `mcp-server/completions/mcp-agent.fish` - Fish completions

## Dependencies
```toml
[[bin]]
name = "mcp-agent"
path = "src/bin/mcp-agent.rs"

[dependencies]
# Existing dependencies...
clap = { version = "4.0", features = ["derive"] }
colored = "2.0"
crossterm = "0.27"
ratatui = "0.24"
prettytable-rs = "0.10"
serde_yaml = "0.9"
dirs = "5.0"
```

## Usage Examples

### Basic Commands
```bash
# Register agent
mcp-agent register --capability rust --capability testing --max-tasks 5

# Start agent
mcp-agent start --poll --auto-accept

# Discover work
mcp-agent discover --limit 5 --capability rust

# List tasks
mcp-agent task list --state in_progress

# Accept a task
mcp-agent task accept TASK-001

# Send a message
mcp-agent message send --task TASK-001 --msg-type update "Completed initial implementation"

# Request help
mcp-agent help request --task TASK-001 --help-type technical_question "How to handle edge case X?"
```

### Interactive Mode
```bash
# Start interactive TUI
mcp-agent interactive

# Commands in interactive mode:
# Tab/Shift-Tab - Switch between tabs
# Arrow keys - Navigate
# Enter - Select
# a - Accept selected task
# c - Complete selected task
# r - Refresh data
# : - Enter command mode
# q - Quit
```

## Configuration File
`~/.mcp-agent/config.toml`:
```toml
default_agent = "rust-developer"
server_url = "http://localhost:8080"

[preferences]
auto_refresh_seconds = 30
default_format = "table"
max_messages_display = 50

[shortcuts]
accept = "a"
complete = "c"
refresh = "r"
```

## Notes
- Rich CLI with colored output
- Interactive TUI mode for continuous work
- Shell completions for better UX
- Multiple output formats (table, json, yaml, plain)
- Configuration file support
- Real-time updates in interactive mode