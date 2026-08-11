mod backend;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap},
    Frame, Terminal,
};
use std::io;

const TITLE: &str = " Ubuntu Optimizer & Debloater ";

#[derive(Clone, Copy, PartialEq)]
enum View {
    Overview,
    Packages,
    Privacy,
    Performance,
    Desktop,
}

impl View {
    const ALL: [View; 5] = [
        View::Overview,
        View::Packages,
        View::Privacy,
        View::Performance,
        View::Desktop,
    ];
    fn label(self) -> &'static str {
        match self {
            View::Overview => "Overview",
            View::Packages => "Packages",
            View::Privacy => "Privacy",
            View::Performance => "Performance",
            View::Desktop => "Desktop",
        }
    }
    fn rows(self) -> Vec<&'static str> {
        match self {
            View::Overview => vec!["remove_snap", "telemetry_off", "swappiness_tuned"],
            View::Packages => vec!["install_flatpak", "firefox_ppa"],
            View::Privacy => vec!["apport_off", "motd_off"],
            View::Performance => vec!["shutdown_fast", "ssd_trim"],
            View::Desktop => vec!["tracker_off", "baloo_off", "bloat_removed"],
        }
    }
}

const META: &[(&str, &str, &str)] = &[
    ("remove_snap", "Remove Snap", "Uninstalls snapd, replaces snap apps with flatpak/apt"),
    ("install_flatpak", "Install Flatpak", "Adds the open Flathub ecosystem"),
    ("firefox_ppa", "Native Firefox", "Uses Mozilla's native repo instead of Snap"),
    ("telemetry_off", "Disable telemetry", "Stops ubuntu-report, popcon, geoclue"),
    ("apport_off", "Disable crash popups", "Turns off apport and whoopsie"),
    ("motd_off", "Remove terminal ads", "Disables MOTD news and Ubuntu Pro banners"),
    ("swappiness_tuned", "Tune swappiness", "Sets vm.swappiness=10 (prefer RAM over swap)"),
    ("shutdown_fast", "Shorten shutdown", "systemd timeout 10s instead of 90s"),
    ("ssd_trim", "Enable SSD TRIM", "Activates fstrim.timer"),
    ("tracker_off", "Disable file indexing", "Stops GNOME Tracker (GNOME only)"),
    ("baloo_off", "Disable Baloo indexing", "Stops KDE Baloo (KDE only)"),
    ("bloat_removed", "Remove desktop bloat", "Removes optional games and extras"),
];

struct App {
    info: backend::SystemInfo,
    scan: backend::ScanResult,
    selected: std::collections::HashMap<String, bool>,
    view: View,
    list_index: usize,
    log: Vec<String>,
    log_scroll: u16,
}

impl App {
    fn new() -> App {
        let info = backend::detect_system();
        let scan = backend::scan_system();
        let log0 = format!("System: {} | Desktop: {}", info.distro, info.desktop);
        App {
            info,
            scan,
            selected: std::collections::HashMap::new(),
            view: View::Overview,
            list_index: 0,
            log: vec![log0],
            log_scroll: 0,
        }
    }

    fn visible_rows(&self) -> Vec<&'static str> {
        self.view
            .rows()
            .into_iter()
            .filter(|k| {
                if *k == "tracker_off" {
                    return self.info.is_gnome;
                }
                if *k == "baloo_off" {
                    return self.info.is_kde;
                }
                true
            })
            .collect()
    }

    fn applied_count(&self) -> usize {
        let s = &self.scan;
        [
            s.remove_snap,
            s.install_flatpak,
            s.firefox_ppa,
            s.telemetry_off,
            s.apport_off,
            s.motd_off,
            s.swappiness_tuned,
            s.shutdown_fast,
            s.ssd_trim,
            s.tracker_off,
            s.baloo_off,
            s.bloat_removed,
        ]
        .iter()
        .filter(|b| **b)
        .count()
    }

    fn is_applied(&self, key: &str) -> bool {
        match key {
            "remove_snap" => self.scan.remove_snap,
            "install_flatpak" => self.scan.install_flatpak,
            "firefox_ppa" => self.scan.firefox_ppa,
            "telemetry_off" => self.scan.telemetry_off,
            "apport_off" => self.scan.apport_off,
            "motd_off" => self.scan.motd_off,
            "swappiness_tuned" => self.scan.swappiness_tuned,
            "shutdown_fast" => self.scan.shutdown_fast,
            "ssd_trim" => self.scan.ssd_trim,
            "tracker_off" => self.scan.tracker_off,
            "baloo_off" => self.scan.baloo_off,
            "bloat_removed" => self.scan.bloat_removed,
            _ => false,
        }
    }

    fn toggle_current(&mut self) {
        let rows = self.visible_rows();
        if rows.is_empty() {
            return;
        }
        let key = rows[self.list_index.min(rows.len() - 1)];
        if self.is_applied(key) {
            return; // already applied — no-op
        }
        let entry = self.selected.entry(key.to_string()).or_insert(false);
        *entry = !*entry;
    }

    fn scan_now(&mut self) {
        self.scan = backend::scan_system();
        self.log
            .push(format!("Scan complete. {} tweaks already applied.", self.applied_count()));
        self.log_scroll = self.log.len().saturating_sub(1) as u16;
    }

    fn apply_selected(&mut self) {
        let pending: Vec<&str> = self
            .visible_rows()
            .into_iter()
            .filter(|k| *self.selected.get(*k).unwrap_or(&false))
            .collect();
        if pending.is_empty() {
            self.log.push("Nothing selected to apply.".into());
            return;
        }
        let body = build_script(&pending, &self.info);
        self.log.push(format!("Applying: {}", pending.join(", ")));
        match backend::run_optimizer_script(&body) {
            Ok(out) => {
                self.log.push("Done. All selected optimizations applied.".into());
                for line in out.lines().take(3) {
                    self.log.push(format!("  {line}"));
                }
                self.selected.clear();
                self.scan_now();
            }
            Err(e) => {
                self.log.push(format!("ERROR: {e}"));
            }
        }
        self.log_scroll = self.log.len().saturating_sub(1) as u16;
    }

    fn restore(&mut self) {
        let script = "\
echo '>>> Restoring default settings...'
rm -f /etc/apt/preferences.d/nosnap.pref
sysctl -w vm.swappiness=60
sed -i 's/vm.swappiness=.*/vm.swappiness=60/' /etc/sysctl.conf || true
sed -i 's/DefaultTimeoutStopSec=.*/DefaultTimeoutStopSec=90s/' /etc/systemd/system.conf || true
sed -i 's/enabled=0/enabled=1/g' /etc/default/apport || true
systemctl enable apport 2>/dev/null || true
echo '>>> Defaults restored.'";
        self.log.push("Restoring default system settings...".into());
        match backend::run_optimizer_script(script) {
            Ok(out) => {
                self.log.push("Defaults restored.".into());
                for line in out.lines().take(3) {
                    self.log.push(format!("  {line}"));
                }
                self.scan_now();
            }
            Err(e) => self.log.push(format!("ERROR: {e}")),
        }
        self.log_scroll = self.log.len().saturating_sub(1) as u16;
    }

    fn handle_key(&mut self, key: KeyCode) {
        let rows = self.visible_rows();
        let len = rows.len();
        match key {
            KeyCode::Char('q') => {
                if self.view == View::Overview {
                    std::process::exit(0);
                }
            }
            KeyCode::Esc => self.view = View::Overview,
            KeyCode::Up | KeyCode::Char('k') => {
                if len > 0 {
                    self.list_index = (self.list_index + len - 1) % len;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 {
                    self.list_index = (self.list_index + 1) % len;
                }
            }
            KeyCode::Tab => {
                let idx = View::ALL.iter().position(|v| *v == self.view).unwrap();
                self.view = View::ALL[(idx + 1) % View::ALL.len()];
                self.list_index = 0;
            }
            KeyCode::Char('1') => self.view = View::Overview,
            KeyCode::Char('2') => self.view = View::Packages,
            KeyCode::Char('3') => self.view = View::Privacy,
            KeyCode::Char('4') => self.view = View::Performance,
            KeyCode::Char('5') => self.view = View::Desktop,
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('a') => self.apply_selected(),
            KeyCode::Char('s') => self.scan_now(),
            KeyCode::Char('r') => self.restore(),
            _ => {}
        }
        if len > 0 && self.list_index >= len {
            self.list_index = len - 1;
        }
    }
}

fn build_script(keys: &[&str], info: &backend::SystemInfo) -> String {
    let mut s = String::new();
    s.push_str("echo '>>> Ubuntu Optimizer starting...'\n");
    for k in keys {
        match *k {
            "remove_snap" => s.push_str(
                "systemctl stop snapd.service snapd.socket snapd.seeded.service 2>/dev/null || true\n\
                 systemctl disable snapd.service snapd.socket snapd.seeded.service 2>/dev/null || true\n\
                 apt-get purge -y snapd 2>/dev/null || apt purge -y snapd 2>/dev/null || true\n\
                 rm -rf /var/snap /var/lib/snapd /var/cache/snapd ~/snap\n\
                 printf 'Package: snapd\\nPin: release *\\nPin-Priority: -10\\n' > /etc/apt/preferences.d/nosnap.pref\n",
            ),
            "install_flatpak" => s.push_str(
                "apt-get update -qq && apt-get install -y flatpak 2>/dev/null || true\n\
                 flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo 2>/dev/null || true\n",
            ),
            "firefox_ppa" => s.push_str(
                "install -d -m 0755 /etc/apt/keyrings\n\
                 wget -q https://packages.mozilla.org/apt/repo-signing-key.gpg -O /etc/apt/keyrings/packages.mozilla.org.asc 2>/dev/null || true\n\
                 echo 'deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main' > /etc/apt/sources.list.d/mozilla.list\n\
                 printf 'Package: *\\nPin: origin packages.mozilla.org\\nPin-Priority: 1000\\n' > /etc/apt/preferences.d/mozilla\n\
                 apt-get update -qq && apt-get install -y firefox 2>/dev/null || true\n",
            ),
            "telemetry_off" => s.push_str(
                "apt-get purge -y ubuntu-report popularity-contest geoclue 2>/dev/null || true\n",
            ),
            "apport_off" => s.push_str(
                "systemctl stop apport whoopsie 2>/dev/null || true\n\
                 systemctl disable apport whoopsie 2>/dev/null || true\n\
                 sed -i 's/enabled=1/enabled=0/g' /etc/default/apport 2>/dev/null || true\n",
            ),
            "motd_off" => s.push_str(
                "chmod -x /etc/update-motd.d/50-motd-news /etc/update-motd.d/80-livepatch 2>/dev/null || true\n\
                 sed -i 's/ENABLED=1/ENABLED=0/g' /etc/default/motd-news 2>/dev/null || true\n",
            ),
            "swappiness_tuned" => s.push_str(
                "sysctl -w vm.swappiness=10\n\
                 grep -q 'vm.swappiness' /etc/sysctl.conf && sed -i 's/vm.swappiness=.*/vm.swappiness=10/' /etc/sysctl.conf || echo 'vm.swappiness=10' >> /etc/sysctl.conf\n",
            ),
            "shutdown_fast" => s.push_str(
                "grep -q 'DefaultTimeoutStopSec' /etc/systemd/system.conf && sed -i 's/.*DefaultTimeoutStopSec=.*/DefaultTimeoutStopSec=10s/' /etc/systemd/system.conf || echo 'DefaultTimeoutStopSec=10s' >> /etc/systemd/system.conf\n",
            ),
            "ssd_trim" => s.push_str("systemctl enable --now fstrim.timer 2>/dev/null || true\n"),
            "tracker_off" if info.is_gnome => s.push_str(
                "systemctl --user stop tracker-miner-fs-3.service tracker-extract-3.service 2>/dev/null || true\n\
                 systemctl --user mask tracker-miner-fs-3.service tracker-extract-3.service 2>/dev/null || true\n",
            ),
            "baloo_off" if info.is_kde => s.push_str("balooctl disable 2>/dev/null || balooctl purge 2>/dev/null || true\n"),
            "bloat_removed" => s.push_str(
                "apt-get purge -y aisleriot gnome-mahjongg gnome-mines gnome-sudoku shotwell kmines ksudoku 2>/dev/null || true\n\
                 apt-get autoremove -y 2>/dev/null || true\n",
            ),
            _ => {}
        }
    }
    s.push_str("echo '>>> Done.'\n");
    s
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // body
            Constraint::Length(6),  // log
            Constraint::Length(1),  // status bar
        ])
        .split(area);

    render_header(f, chunks[0], app);
    render_body(f, chunks[1], app);
    render_log(f, chunks[2], app);
    render_status(f, chunks[3], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let de = if app.info.is_kde {
        "KDE Plasma"
    } else if app.info.is_gnome {
        "GNOME"
    } else {
        "Desktop not detected"
    };
    let applied = app.applied_count();
    let total = 12;
    let line = Line::from(vec![
        Span::styled(
            TITLE,
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {} ", app.info.distro),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {de} "),
            Style::default().fg(Color::Black).bg(Color::Magenta),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" health {applied}/{total} "),
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn render_body(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(0)])
        .split(area);

    // Sidebar: views + actions
    let mut items: Vec<ListItem> = View::ALL
        .iter()
        .map(|v| {
            let label = v.label();
            let active = *v == app.view;
            ListItem::new(Line::from(vec![Span::styled(
                format!("{label}"),
                if active {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )]))
        })
        .collect();
    items.push(ListItem::new(Line::from(vec![Span::raw("")])));
    items.push(ListItem::new(Line::from(vec![Span::styled(
        " s  scan",
        Style::default().fg(Color::Yellow),
    )])));
    items.push(ListItem::new(Line::from(vec![Span::styled(
        " a  apply",
        Style::default().fg(Color::Green),
    )])));
    items.push(ListItem::new(Line::from(vec![Span::styled(
        " r  restore",
        Style::default().fg(Color::Red),
    )])));
    items.push(ListItem::new(Line::from(vec![Span::styled(
        " q  quit",
        Style::default().fg(Color::Gray),
    )])));

    let mut state = ListState::default();
    let view_idx = View::ALL.iter().position(|v| *v == app.view).unwrap();
    state.select(Some(view_idx));

    let sidebar = List::new(items)
        .block(
            Block::default()
                .title(" Menu ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(sidebar, cols[0], &mut state);

    // Main table
    let rows = app.visible_rows();
    let header = Row::new(vec![
        Cell::from(Span::styled(
            " Option ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Status",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Description",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
    ]);

    let table_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let (_, label, desc) = META.iter().find(|(key, _, _)| key == k).unwrap();
            let applied = app.is_applied(k);
            let selected = *app.selected.get(*k).unwrap_or(&false);

            let status = if applied {
                Span::styled(" ✓ Applied ", Style::default().fg(Color::Green))
            } else if selected {
                Span::styled(" ▣ Selected ", Style::default().fg(Color::Cyan))
            } else {
                Span::styled(" ○ Pending ", Style::default().fg(Color::Gray))
            };

            let marker = if applied {
                "[✓]"
            } else if selected {
                "[▣]"
            } else {
                "[ ]"
            };

            let row_style = if i == app.list_index {
                Style::default().bg(Color::Rgb(38, 42, 57))
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    format!(" {marker} {label} "),
                    Style::default()
                        .fg(if applied { Color::Green } else { Color::White })
                        .add_modifier(if applied { Modifier::BOLD } else { Modifier::empty() }),
                )),
                Cell::from(status),
                Cell::from(Span::styled(*desc, Style::default().fg(Color::Gray))),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        table_rows,
        [
            Constraint::Length(30),
            Constraint::Length(12),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!(" {} ", app.view.label()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(table, cols[1]);
}

fn render_log(f: &mut Frame, area: Rect, app: &App) {
    let content: Vec<Line> = app
        .log
        .iter()
        .map(|l| {
            if l.starts_with("ERROR") {
                Line::from(Span::styled(l, Style::default().fg(Color::Red)))
            } else if l.starts_with("  ") {
                Line::from(Span::styled(l, Style::default().fg(Color::DarkGray)))
            } else {
                Line::from(Span::styled(
                    l,
                    Style::default().fg(Color::Rgb(134, 239, 172)),
                ))
            }
        })
        .collect();
    let p = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Activity ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .scroll((app.log_scroll, 0))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let status = format!(
        " [↑/↓] move  [Space/Enter] toggle  [Tab/1-5] view  [a] apply  [s] scan  [r] restore  [q] quit "
    );
    let p = Paragraph::new(status)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::Black).bg(Color::DarkGray));
    f.render_widget(p, area);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_app(&mut terminal)
    }));

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(Box::new(io::Error::other(e.to_string()))),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    app.log
        .push("Press [s] to scan, [Space] to select, [a] to apply.".into());
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }
                app.handle_key(key.code);
            }
        }
    }
}
