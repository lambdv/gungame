/// Server configuration - immutable after load
#[derive(Debug, Clone)]
pub struct Config {
    pub http_port: u16,
    pub udp_port: u16,
    pub tick_rate_hz: u32,
    pub player_inactivity_timeout_secs: u64,
    pub max_lobbies: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            http_port: 8080,
            udp_port: 8081,
            tick_rate_hz: 50, // 20ms per tick
            player_inactivity_timeout_secs: 15,
            max_lobbies: 1000,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick_interval_ms(&self) -> u64 {
        1000 / self.tick_rate_hz as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.http_port, 8080);
        assert_eq!(config.udp_port, 8081);
        assert_eq!(config.tick_rate_hz, 50);
    }

    #[test]
    fn test_tick_interval() {
        let config = Config::default();
        assert_eq!(config.tick_interval_ms(), 20);
    }
}

