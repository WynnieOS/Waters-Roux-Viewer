//! Linden Lab's public Second Life grid health and in-world population.
//! Data is fetched off the UI thread and refreshed on a 3-minute cadence.
//! Missing feeds are displayed as unavailable, never as zero residents.

use std::time::{Duration, Instant};

const STATISTICS: &str = "https://api.secondlife.com/datafeeds/secondlife.xml";
const STATUS_PAGE: &str = "https://status.secondlifegrid.net/api/v2/status.json";
const POLL_EVERY: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Checking,
    Online,
    Degraded,
    Offline,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overview {
    pub health: Health,
    pub residents: Option<u32>,
}

impl Default for Overview {
    fn default() -> Self {
        Self {
            health: Health::Checking,
            residents: None,
        }
    }
}

impl Overview {
    pub fn status(self) -> &'static str {
        match self.health {
            Health::Checking => "Checking…",
            Health::Online => "Online",
            Health::Degraded => "Degraded",
            Health::Offline => "Offline",
            Health::Unavailable => "Unavailable",
        }
    }

    pub fn population(self) -> String {
        self.residents.map_or_else(|| "Unavailable".to_owned(), |n| n.to_string())
    }
}

fn value(xml: &str, key: &str) -> Option<String> {
    let open = format!("<{key}>");
    let close = format!("</{key}>");
    let at = xml.find(&open)? + open.len();
    let end = xml[at..].find(&close)? + at;
    Some(xml[at..end].trim().to_owned())
}

pub fn parse_stats(xml: &str) -> Option<Overview> {
    let status = value(xml, "status")?;
    let health = match status.as_str() {
        "ONLINE" => Health::Online,
        "OFFLINE" => Health::Offline,
        _ => Health::Unavailable,
    };
    let residents = value(xml, "inworld")
        .and_then(|v| v.chars().filter(|c| *c != ',').collect::<String>().parse::<u32>().ok())
        .filter(|v| *v <= 2_000_000);
    Some(Overview { health, residents })
}

pub fn parse_status_page(body: &str) -> Option<Health> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    match json.get("status")?.get("indicator")?.as_str()? {
        "none" => Some(Health::Online),
        "minor" | "major" => Some(Health::Degraded),
        "critical" => Some(Health::Offline),
        _ => Some(Health::Unavailable),
    }
}

async fn fetch() -> Overview {
    let http = match reqwest::Client::builder().timeout(Duration::from_secs(8)).https_only(true).build() {
        Ok(http) => http,
        Err(_) => {
            return Overview {
                health: Health::Unavailable,
                residents: None,
            };
        }
    };
    if let Ok(response) = http.get(STATISTICS).send().await
        && let Ok(response) = response.error_for_status()
        && let Ok(xml) = response.text().await
        && let Some(stats) = parse_stats(&xml)
    {
        return stats;
    }
    // Population and incidents are independent. A broken population feed
    // does not mean that the SL grid itself is offline.
    let health = if let Ok(response) = http.get(STATUS_PAGE).send().await
        && let Ok(response) = response.error_for_status()
        && let Ok(json) = response.text().await
    {
        parse_status_page(&json).unwrap_or(Health::Unavailable)
    } else {
        Health::Unavailable
    };
    Overview { health, residents: None }
}

pub struct LiveGrid {
    pub overview: Overview,
    rt: Option<tokio::runtime::Handle>,
    tx: crossbeam_channel::Sender<Overview>,
    rx: crossbeam_channel::Receiver<Overview>,
    last_requested: Option<Instant>,
}

impl Default for LiveGrid {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::bounded(2);
        Self {
            overview: Overview::default(),
            rt: None,
            tx,
            rx,
            last_requested: None,
        }
    }
}

impl LiveGrid {
    pub fn start(rt: &tokio::runtime::Handle) -> Self {
        let mut feed = Self {
            rt: Some(rt.clone()),
            ..Self::default()
        };
        feed.refresh();
        feed
    }

    fn refresh(&mut self) {
        let Some(rt) = &self.rt else { return };
        let tx = self.tx.clone();
        self.last_requested = Some(Instant::now());
        rt.spawn(async move {
            let _ = tx.try_send(fetch().await);
        });
    }

    pub fn poll(&mut self) {
        while let Ok(overview) = self.rx.try_recv() {
            self.overview = overview;
        }
        if self.last_requested.is_some_and(|start| start.elapsed() >= POLL_EVERY) {
            self.refresh();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_xml_parses_and_never_invents_population() {
        let state = parse_stats("<stats><status>ONLINE</status><inworld>38,836</inworld></stats>").unwrap();
        assert_eq!(state.health, Health::Online);
        assert_eq!(state.residents, Some(38_836));
        assert_eq!(parse_stats("<stats><status>OFFLINE</status></stats>").unwrap().residents, None);
        assert_eq!(Overview::default().population(), "Unavailable");
    }
    #[test]
    fn statuspage_health_maps_degraded_conditions() {
        assert_eq!(parse_status_page(r#"{"status":{"indicator":"minor"}}"#), Some(Health::Degraded));
        assert_eq!(parse_status_page(r#"{"status":{"indicator":"critical"}}"#), Some(Health::Offline));
        assert_eq!(parse_status_page("bad-json"), None);
    }
}