use smallvec::SmallVec;

/// Type alias for small collections that avoid allocations
pub type SmallPlayerVec = SmallVec<[u32; 8]>;
pub type SmallEventVec = SmallVec<[SyncEvent; 16]>;

/// Sync event for delta-based state updates
#[derive(Debug, Clone)]
pub enum SyncEvent {
    HealthChanged { player_id: u32, health: u32 },
    AmmoChanged { player_id: u32, ammo: u32 },
    MaxAmmoChanged { player_id: u32, max_ammo: u32 },
    WeaponChanged { player_id: u32, weapon_id: u32 },
    ReloadStateChanged { player_id: u32, is_reloading: bool },
    PositionChanged { player_id: u32, position: (f32, f32, f32), rotation: (f32, f32, f32) },
}

/// Pre-allocated buffer for packet serialization
pub struct PacketBuffer {
    buffer: Vec<u8>,
}

impl PacketBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }
}

impl Default for PacketBuffer {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_vec_creation() {
        let mut vec: SmallPlayerVec = SmallVec::new();
        vec.push(1);
        vec.push(2);
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn test_packet_buffer() {
        let mut buf = PacketBuffer::new(512);
        buf.clear();
        assert_eq!(buf.as_mut_slice().len(), 0);
    }
}

