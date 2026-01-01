use dashmap::DashMap;
use std::time::SystemTime;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobalPlayerStats {
    pub player_id: u32,
    pub name: String,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub total_score: u32,
    pub games_played: u32,
    pub last_seen: SystemTime,
    pub created_at: SystemTime,
}

impl GlobalPlayerStats {
    pub fn new(player_id: u32, name: String) -> Self {
        Self {
            player_id,
            name,
            total_kills: 0,
            total_deaths: 0,
            total_score: 0,
            games_played: 0,
            last_seen: SystemTime::now(),
            created_at: SystemTime::now(),
        }
    }

    pub fn record_session(&mut self, kills: u32, deaths: u32, score: u32) {
        self.total_kills += kills;
        self.total_deaths += deaths;
        self.total_score += score;
        self.games_played += 1;
        self.last_seen = SystemTime::now();
    }

    pub fn kdratio(&self) -> f32 {
        if self.total_deaths > 0 {
            self.total_kills as f32 / self.total_deaths as f32
        } else {
            self.total_kills as f32
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalStats {
    players: DashMap<u32, GlobalPlayerStats>,
}

impl GlobalStats {
    pub fn new() -> Self {
        Self {
            players: DashMap::new(),
        }
    }

    pub fn record_session(&self, player_id: u32, name: &str, kills: u32, deaths: u32, score: u32) {
        let mut stats = self
            .players
            .entry(player_id)
            .or_insert_with(|| GlobalPlayerStats::new(player_id, name.to_string()));
        stats.name = name.to_string();
        stats.record_session(kills, deaths, score);
    }

    pub fn get_stats(&self, player_id: u32) -> Option<GlobalPlayerStats> {
        self.players.get(&player_id).map(|s| s.clone())
    }

    pub fn get_top_players(&self, limit: usize) -> Vec<GlobalPlayerStats> {
        let mut all: Vec<_> = self
            .players
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        all.sort_by(|a, b| b.total_score.cmp(&a.total_score));
        all.into_iter().take(limit).collect()
    }

    pub fn get_top_by_kills(&self, limit: usize) -> Vec<GlobalPlayerStats> {
        let mut all: Vec<_> = self
            .players
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        all.sort_by(|a, b| b.total_kills.cmp(&a.total_kills));
        all.into_iter().take(limit).collect()
    }

    pub fn cleanup_old_entries(&self, max_age_secs: u64) -> usize {
        let now = SystemTime::now();
        let mut removed = 0;
        let threshold = std::time::Duration::from_secs(max_age_secs);

        let to_remove: Vec<u32> = self
            .players
            .iter()
            .filter_map(|entry| {
                let stats = entry.value();
                if let Ok(duration) = now.duration_since(stats.last_seen) {
                    if duration > threshold && stats.games_played == 0 {
                        return Some(stats.player_id);
                    }
                }
                None
            })
            .collect();

        for player_id in to_remove {
            self.players.remove(&player_id);
            removed += 1;
        }

        removed
    }
}

impl Default for GlobalStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_stats_creation() {
        let stats = GlobalStats::new();
        assert_eq!(stats.players.len(), 0);
    }

    #[test]
    fn test_record_session() {
        let stats = GlobalStats::new();
        stats.record_session(1, "Player1", 5, 2, 500);

        let player_stats = stats.get_stats(1).unwrap();
        assert_eq!(player_stats.total_kills, 5);
        assert_eq!(player_stats.total_deaths, 2);
        assert_eq!(player_stats.total_score, 500);
        assert_eq!(player_stats.games_played, 1);
    }

    #[test]
    fn test_kdratio() {
        let stats = GlobalStats::new();

        stats.record_session(1, "Player1", 10, 5, 1000);
        let player_stats = stats.get_stats(1).unwrap();
        assert!((player_stats.kdratio() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_top_players() {
        let stats = GlobalStats::new();

        stats.record_session(1, "Player1", 100, 50, 10000);
        stats.record_session(2, "Player2", 50, 25, 5000);
        stats.record_session(3, "Player3", 200, 100, 20000);

        let top = stats.get_top_players(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].player_id, 3);
        assert_eq!(top[1].player_id, 1);
    }
}
