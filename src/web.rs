//! Browser adapter: the native Ratatui widgets and statistics run unchanged in Wasm.
use crate::{
    config::Target,
    monitor::{PingResult, TargetStats},
    ui::{App, PlotView, TabMode},
};
use chrono::{TimeZone, Utc};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Demo {
    app: App,
    targets: Vec<TargetStats>,
    sample: u32,
    scenario: u8,
}

#[wasm_bindgen]
impl Demo {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let targets = [
            ("Gateway", "192.0.2.1"),
            ("DNS", "192.0.2.53"),
            ("API", "198.51.100.10"),
            ("Edge", "203.0.113.8"),
        ]
        .into_iter()
        .map(|(name, address)| {
            TargetStats::new(
                Target {
                    address: address.into(),
                    name: Some(name.into()),
                },
                100,
            )
        })
        .collect();
        let mut demo = Self {
            app: App {
                current_tab: 0,
                current_plot_view: PlotView::AllTargets,
                tab_mode: TabMode::AllTargets,
            },
            targets,
            sample: 0,
            scenario: 0,
        };
        // Seed a readable window; subsequent samples arrive once per second.
        for _ in 0..60 {
            demo.tick();
        }
        demo
    }

    pub fn tick(&mut self) {
        let t = self.sample;
        for (i, target) in self.targets.iter_mut().enumerate() {
            let phase = t as f64 * 0.23 + i as f64 * 1.7;
            let jitter = ((t.wrapping_mul(17) + i as u32 * 31) % 13) as f64 / 4.0;
            let base = [3.0, 14.0, 30.0, 49.0][i % 4];
            let failed = match self.scenario {
                1 => i >= 2 && t % 5 < 2,
                2 => i == 2,
                _ => false,
            };
            let spike = if self.scenario == 1 && i >= 2 {
                38.0 + 25.0 * phase.sin().abs()
            } else {
                0.0
            };
            let timestamp = Utc.timestamp_opt(1_789_300_800 + t as i64, 0).unwrap();
            target.add_ping_result(
                PingResult {
                    timestamp,
                    latency_ms: (!failed).then_some(
                        base + phase.sin().abs() * (i + 1) as f64 * 2.0 + jitter + spike,
                    ),
                    success: !failed,
                    failure_reason: failed.then(|| {
                        if self.scenario == 2 {
                            "Host unreachable"
                        } else {
                            "Request timeout"
                        }
                        .into()
                    }),
                    resolved_ip: target.target.address.parse().ok(),
                },
                100,
            );
            // Use deterministic sample time in the simulated failure log too.
            if failed {
                target.failure_log.back_mut().unwrap().timestamp = timestamp;
            }
        }
        self.sample += 1;
    }

    /// Add a named simulation target without opening any network connection.
    pub fn add_target(&mut self, address: &str, name: &str) -> Result<(), JsValue> {
        if self.targets.len() >= 16 {
            return Err(JsValue::from_str("Up to 16 hosts can be added."));
        }
        if address.is_empty()
            || address.len() > 253
            || !address.is_ascii()
            || address.chars().any(|c| c.is_whitespace() || c.is_control())
            || name.is_empty()
            || name.len() > 40
            || !name.is_ascii()
            || name.chars().any(|c| c.is_control())
        {
            return Err(JsValue::from_str(
                "Use a valid host and a name of 1–40 plain-text characters.",
            ));
        }
        if self
            .targets
            .iter()
            .any(|target| target.target.address.eq_ignore_ascii_case(address))
        {
            return Err(JsValue::from_str("That host is already in the monitor."));
        }
        self.targets.push(TargetStats::new(
            Target {
                address: address.into(),
                name: Some(name.into()),
            },
            100,
        ));
        self.app.current_plot_view = PlotView::PingOnly;
        self.app.current_tab = self.targets.len();
        self.app.tab_mode = TabMode::Individual(self.targets.len() - 1);
        self.tick();
        Ok(())
    }

    pub fn set_scenario(&mut self, scenario: u8) {
        self.scenario = scenario.min(2);
    }
    pub fn samples(&self) -> u32 {
        self.sample
    }
    pub fn key(&mut self, key: &str) {
        match key {
            "next" => self.app.next_tab(self.targets.len()),
            "previous" => self.app.previous_tab(self.targets.len()),
            "plot" => self.app.next_plot_view(),
            _ => {}
        }
    }

    /// Serialize styled text runs, keeping the actual Ratatui layout and braille charts.
    pub fn render(&self, columns: u16, rows: u16) -> String {
        let columns = columns.clamp(80, 180);
        let rows = rows.clamp(32, 60);
        let mut terminal = Terminal::new(TestBackend::new(columns, rows)).unwrap();
        terminal
            .draw(|frame| crate::ui::ui(frame, &self.app, &self.targets))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut lines = Vec::new();
        for y in 0..rows {
            let mut runs: Vec<(String, String, String, bool)> = Vec::new();
            for x in 0..columns {
                let cell = &buffer[(x, y)];
                let fg = color(cell.fg);
                let bg = color(cell.bg);
                let bold = cell.modifier.contains(Modifier::BOLD);
                if let Some(last) = runs
                    .last_mut()
                    .filter(|last| last.1 == fg && last.2 == bg && last.3 == bold)
                {
                    last.0.push_str(cell.symbol());
                } else {
                    runs.push((cell.symbol().into(), fg, bg, bold));
                }
            }
            lines.push(runs);
        }
        serde_json::to_string(&lines).unwrap()
    }
}

fn color(color: Color) -> String {
    match color {
        Color::Reset => "inherit".into(),
        Color::Rgb(r, g, b) => format!("rgb({r},{g},{b})"),
        Color::Indexed(i) => format!("var(--c{})", i % 16),
        c => format!(
            "var(--c{})",
            match c {
                Color::Black => 0,
                Color::Red => 1,
                Color::Green => 2,
                Color::Yellow => 3,
                Color::Blue => 4,
                Color::Magenta => 5,
                Color::Cyan => 6,
                Color::Gray => 7,
                Color::DarkGray => 8,
                Color::LightRed => 9,
                Color::LightGreen => 10,
                Color::LightYellow => 11,
                Color::LightBlue => 12,
                Color::LightMagenta => 13,
                Color::LightCyan => 14,
                _ => 15,
            }
        ),
    }
}
