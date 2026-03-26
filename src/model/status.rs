use super::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MotionState {
    #[default]
    Idle,
    Moving,
    Tracking,
    Parking,
    Error,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeviceStatus {
    pub position: Option<Position>,
    pub target_position: Option<Position>,
    pub motion_state: MotionState,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
}

impl DeviceStatus {
    pub fn with_position(position: Position) -> Self {
        Self {
            position: Some(position),
            ..Default::default()
        }
    }

    pub fn ok() -> Self {
        Self::default()
    }

    pub fn error(code: i32, message: impl Into<String>) -> Self {
        Self {
            motion_state: MotionState::Error,
            error_code: Some(code),
            error_message: Some(message.into()),
            ..Default::default()
        }
    }
}
