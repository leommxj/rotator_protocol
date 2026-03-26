use super::position::Position;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Speed {
    Normalized(f64),
    Absolute(f64),
}

impl Default for Speed {
    fn default() -> Self {
        Speed::Normalized(0.5)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnifiedCommand {
    GetPosition,
    GotoPosition(Position),
    Move {
        direction: MoveDirection,
        speed: Speed,
    },
    Stop,
    StopAxis1,
    StopAxis2,
    Park,
    SetSpeed(Speed),
    GetInfo,
    Reset,
    SetPreset(u8),
    GotoPreset(u8),
    ClearPreset(u8),
    Raw(Vec<u8>),
}

impl UnifiedCommand {
    pub fn move_up(speed: Speed) -> Self {
        Self::Move {
            direction: MoveDirection::Up,
            speed,
        }
    }

    pub fn move_down(speed: Speed) -> Self {
        Self::Move {
            direction: MoveDirection::Down,
            speed,
        }
    }

    pub fn move_left(speed: Speed) -> Self {
        Self::Move {
            direction: MoveDirection::Left,
            speed,
        }
    }

    pub fn move_right(speed: Speed) -> Self {
        Self::Move {
            direction: MoveDirection::Right,
            speed,
        }
    }
}
