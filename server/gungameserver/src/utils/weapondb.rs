use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Weapon data structure matching client weapon.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponData {
    pub id: u32,
    pub name: String,
    pub damage: u32,
    pub fire_rate: f32,
    pub range: f32,
    pub reload_time: f32,
    pub ammo: u32,
}

/// Immutable weapon database - loaded once at startup
/// Zero contention, passed by Arc reference
#[derive(Debug, Clone)]
pub struct WeaponDb {
    weapons: HashMap<u32, WeaponData>,
}

impl WeaponDb {
    /// Load weapon database with hardcoded data
    /// In production, this would load from a config file
    pub fn load() -> Self {
        let mut weapons = HashMap::new();

        weapons.insert(1, WeaponData {
            id: 1,
            name: "Golden Friend".to_string(),
            damage: 20,
            fire_rate: 4.0,
            range: 100.0,
            reload_time: 1.0,
            ammo: 20,
        });

        weapons.insert(2, WeaponData {
            id: 2,
            name: "Prototype".to_string(),
            damage: 30,
            fire_rate: 2.0,
            range: 150.0,
            reload_time: 1.5,
            ammo: 8,
        });

        weapons.insert(3, WeaponData {
            id: 3,
            name: "Combat Knife".to_string(),
            damage: 50,
            fire_rate: 1.5,
            range: 3.0,
            reload_time: 0.0,
            ammo: 0, // Melee weapon, no ammo limit
        });

        Self { weapons }
    }

    /// Get weapon by ID
    pub fn get(&self, id: u32) -> Option<&WeaponData> {
        self.weapons.get(&id)
    }

    /// Check if weapon exists
    pub fn contains(&self, id: u32) -> bool {
        self.weapons.contains_key(&id)
    }

    /// Get default weapon ID (Golden Friend)
    pub fn default_weapon_id() -> u32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weapon_db_load() {
        let db = WeaponDb::load();
        assert_eq!(db.weapons.len(), 3);
    }

    #[test]
    fn test_weapon_get() {
        let db = WeaponDb::load();
        let weapon = db.get(1);
        assert!(weapon.is_some());
        assert_eq!(weapon.unwrap().name, "Golden Friend");
    }

    #[test]
    fn test_weapon_contains() {
        let db = WeaponDb::load();
        assert!(db.contains(1));
        assert!(db.contains(2));
        assert!(db.contains(3));
        assert!(!db.contains(999));
    }

    #[test]
    fn test_default_weapon_id() {
        assert_eq!(WeaponDb::default_weapon_id(), 1);
    }

    #[test]
    fn test_weapon_data_integrity() {
        let db = WeaponDb::load();
        let knife = db.get(3).unwrap();
        assert_eq!(knife.ammo, 0);
        assert_eq!(knife.reload_time, 0.0);
        assert_eq!(knife.damage, 50);
    }
}

