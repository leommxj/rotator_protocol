use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Reader;
use quick_xml::Writer;
use std::io::Cursor;

use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{
    CoordinateSystem, DeviceStatus, MotionState, MoveDirection, Position, Speed, UnifiedCommand,
};

const DEFAULT_DEVICE: &str = "Rotator";

pub struct IndiCodec {
    buffer: Vec<u8>,
    device_name: String,
}

impl IndiCodec {
    pub fn new() -> Self {
        Self::with_device(DEFAULT_DEVICE)
    }

    pub fn with_device(device_name: &str) -> Self {
        Self {
            buffer: Vec::new(),
            device_name: device_name.to_string(),
        }
    }

    fn build_new_number_vector(&self, property: &str, values: &[(&str, f64)]) -> Result<Vec<u8>> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        let mut elem = BytesStart::new("newNumberVector");
        elem.push_attribute(("device", self.device_name.as_str()));
        elem.push_attribute(("name", property));
        writer
            .write_event(Event::Start(elem))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        for (name, value) in values {
            let mut num_elem = BytesStart::new("oneNumber");
            num_elem.push_attribute(("name", *name));
            writer
                .write_event(Event::Start(num_elem))
                .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
            writer
                .write_event(Event::Text(BytesText::new(&format!("{:.6}", value))))
                .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
            writer
                .write_event(Event::End(BytesEnd::new("oneNumber")))
                .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("newNumberVector")))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        Ok(writer.into_inner().into_inner())
    }

    fn build_new_switch_vector(&self, property: &str, switch: &str, on: bool) -> Result<Vec<u8>> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        let mut elem = BytesStart::new("newSwitchVector");
        elem.push_attribute(("device", self.device_name.as_str()));
        elem.push_attribute(("name", property));
        writer
            .write_event(Event::Start(elem))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        let mut switch_elem = BytesStart::new("oneSwitch");
        switch_elem.push_attribute(("name", switch));
        writer
            .write_event(Event::Start(switch_elem))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
        writer
            .write_event(Event::Text(BytesText::new(if on { "On" } else { "Off" })))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
        writer
            .write_event(Event::End(BytesEnd::new("oneSwitch")))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        writer
            .write_event(Event::End(BytesEnd::new("newSwitchVector")))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        Ok(writer.into_inner().into_inner())
    }

    fn build_get_properties(&self) -> Result<Vec<u8>> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        let mut elem = BytesStart::new("getProperties");
        elem.push_attribute(("version", "1.7"));
        if !self.device_name.is_empty() {
            elem.push_attribute(("device", self.device_name.as_str()));
        }
        writer
            .write_event(Event::Empty(elem))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        Ok(writer.into_inner().into_inner())
    }

    fn build_set_number_vector(
        &self,
        property: &str,
        values: &[(&str, f64)],
        state: &str,
    ) -> Result<Vec<u8>> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        let mut elem = BytesStart::new("setNumberVector");
        elem.push_attribute(("device", self.device_name.as_str()));
        elem.push_attribute(("name", property));
        elem.push_attribute(("state", state));
        writer
            .write_event(Event::Start(elem))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        for (name, value) in values {
            let mut num_elem = BytesStart::new("oneNumber");
            num_elem.push_attribute(("name", *name));
            writer
                .write_event(Event::Start(num_elem))
                .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
            writer
                .write_event(Event::Text(BytesText::new(&format!("{:.6}", value))))
                .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
            writer
                .write_event(Event::End(BytesEnd::new("oneNumber")))
                .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("setNumberVector")))
            .map_err(|e| Error::Codec(format!("XML write error: {}", e)))?;

        Ok(writer.into_inner().into_inner())
    }

    fn parse_number_values(xml: &str) -> Vec<(String, f64)> {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut values = Vec::new();
        let mut current_name = String::new();
        let mut in_number = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"oneNumber" || name.as_ref() == b"defNumber" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                current_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        in_number = true;
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_number && !current_name.is_empty() {
                        if let Ok(v) = e.unescape().unwrap_or_default().trim().parse::<f64>() {
                            values.push((current_name.clone(), v));
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"oneNumber" || name.as_ref() == b"defNumber" {
                        in_number = false;
                        current_name.clear();
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        values
    }

    fn parse_switch_values(xml: &str) -> Vec<(String, bool)> {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut values = Vec::new();
        let mut current_name = String::new();
        let mut in_switch = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"oneSwitch" || name.as_ref() == b"defSwitch" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                current_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        in_switch = true;
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_switch && !current_name.is_empty() {
                        let text = e.unescape().unwrap_or_default().trim().to_lowercase();
                        let on = text == "on" || text == "1" || text == "true";
                        values.push((current_name.clone(), on));
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"oneSwitch" || name.as_ref() == b"defSwitch" {
                        in_switch = false;
                        current_name.clear();
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        values
    }

    fn get_property_name(xml: &str) -> Option<String> {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"name" {
                            return Some(String::from_utf8_lossy(&attr.value).to_string());
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        None
    }

    fn get_state(xml: &str) -> Option<String> {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"state" {
                            return Some(String::from_utf8_lossy(&attr.value).to_string());
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        None
    }

    fn parse_xml_element(xml: &str, tag_name: &str) -> Result<DecodeResult> {
        match tag_name {
            // Status updates from server
            "setNumberVector" | "defNumberVector" => {
                let property = Self::get_property_name(xml);
                let values = Self::parse_number_values(xml);
                let state_str = Self::get_state(xml).unwrap_or_else(|| "Idle".to_string());

                let motion_state = match state_str.as_str() {
                    "Busy" => MotionState::Moving,
                    "Alert" => MotionState::Error,
                    _ => MotionState::Idle,
                };

                let position = match property.as_deref() {
                    Some("EQUATORIAL_EOD_COORD") => {
                        let ra = values
                            .iter()
                            .find(|(n, _)| n == "RA")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        let dec = values
                            .iter()
                            .find(|(n, _)| n == "DEC")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        Some(Position::new_equatorial(ra, dec))
                    }
                    Some("HORIZONTAL_COORD") => {
                        let az = values
                            .iter()
                            .find(|(n, _)| n == "AZ")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        let alt = values
                            .iter()
                            .find(|(n, _)| n == "ALT")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        Some(Position::new_altaz(az, alt))
                    }
                    Some("ABS_ROTATOR_ANGLE") => {
                        let angle = values
                            .iter()
                            .find(|(n, _)| n == "ANGLE")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        Some(Position::new_altaz(angle, 0.0))
                    }
                    _ => None,
                };

                Ok(DecodeResult::Status(DeviceStatus {
                    position,
                    target_position: None,
                    motion_state,
                    error_code: None,
                    error_message: None,
                }))
            }

            // Commands from client
            "newNumberVector" => {
                let property = Self::get_property_name(xml);
                let values = Self::parse_number_values(xml);

                match property.as_deref() {
                    Some("EQUATORIAL_EOD_COORD") => {
                        let ra = values
                            .iter()
                            .find(|(n, _)| n == "RA")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        let dec = values
                            .iter()
                            .find(|(n, _)| n == "DEC")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                            Position::new_equatorial(ra, dec),
                        )))
                    }
                    Some("HORIZONTAL_COORD") => {
                        let az = values
                            .iter()
                            .find(|(n, _)| n == "AZ")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        let alt = values
                            .iter()
                            .find(|(n, _)| n == "ALT")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                            Position::new_altaz(az, alt),
                        )))
                    }
                    Some("ABS_ROTATOR_ANGLE") => {
                        let angle = values
                            .iter()
                            .find(|(n, _)| n == "ANGLE")
                            .map(|(_, v)| *v)
                            .unwrap_or(0.0);
                        Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                            Position::new_altaz(angle, 0.0),
                        )))
                    }
                    _ => Ok(DecodeResult::None),
                }
            }

            "newSwitchVector" => {
                let property = Self::get_property_name(xml);
                let values = Self::parse_switch_values(xml);

                match property.as_deref() {
                    Some("TELESCOPE_ABORT_MOTION") => {
                        if values.iter().any(|(n, v)| n == "ABORT" && *v) {
                            Ok(DecodeResult::Command(UnifiedCommand::Stop))
                        } else {
                            Ok(DecodeResult::None)
                        }
                    }
                    Some("TELESCOPE_PARK") => {
                        if values.iter().any(|(n, v)| n == "PARK" && *v) {
                            Ok(DecodeResult::Command(UnifiedCommand::Park))
                        } else {
                            Ok(DecodeResult::None)
                        }
                    }
                    Some("TELESCOPE_MOTION_NS") => {
                        if values.iter().any(|(n, v)| n == "MOTION_NORTH" && *v) {
                            Ok(DecodeResult::Command(UnifiedCommand::Move {
                                direction: MoveDirection::Up,
                                speed: Speed::Normalized(1.0),
                            }))
                        } else if values.iter().any(|(n, v)| n == "MOTION_SOUTH" && *v) {
                            Ok(DecodeResult::Command(UnifiedCommand::Move {
                                direction: MoveDirection::Down,
                                speed: Speed::Normalized(1.0),
                            }))
                        } else {
                            Ok(DecodeResult::Command(UnifiedCommand::Stop))
                        }
                    }
                    Some("TELESCOPE_MOTION_WE") => {
                        if values.iter().any(|(n, v)| n == "MOTION_WEST" && *v) {
                            Ok(DecodeResult::Command(UnifiedCommand::Move {
                                direction: MoveDirection::Left,
                                speed: Speed::Normalized(1.0),
                            }))
                        } else if values.iter().any(|(n, v)| n == "MOTION_EAST" && *v) {
                            Ok(DecodeResult::Command(UnifiedCommand::Move {
                                direction: MoveDirection::Right,
                                speed: Speed::Normalized(1.0),
                            }))
                        } else {
                            Ok(DecodeResult::Command(UnifiedCommand::Stop))
                        }
                    }
                    Some("CONNECTION") => {
                        // Just acknowledge connection changes
                        Ok(DecodeResult::None)
                    }
                    _ => Ok(DecodeResult::None),
                }
            }

            "getProperties" => Ok(DecodeResult::Command(UnifiedCommand::GetPosition)),

            "setSwitchVector" | "defSwitchVector" => {
                // Status updates for switches - usually connection state
                Ok(DecodeResult::None)
            }

            _ => Ok(DecodeResult::None),
        }
    }
}

impl Default for IndiCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for IndiCodec {
    fn name(&self) -> &'static str {
        "indi"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        match cmd {
            UnifiedCommand::GetPosition => self.build_get_properties(),
            UnifiedCommand::GotoPosition(pos) => {
                if pos.coord_system == CoordinateSystem::Equatorial {
                    self.build_new_number_vector(
                        "EQUATORIAL_EOD_COORD",
                        &[("RA", pos.ra()), ("DEC", pos.dec())],
                    )
                } else {
                    // For horizontal coordinates or rotator angle
                    if pos.elevation() == 0.0 {
                        // Simple rotator - just azimuth/angle
                        self.build_new_number_vector(
                            "ABS_ROTATOR_ANGLE",
                            &[("ANGLE", pos.azimuth())],
                        )
                    } else {
                        self.build_new_number_vector(
                            "HORIZONTAL_COORD",
                            &[("AZ", pos.azimuth()), ("ALT", pos.elevation())],
                        )
                    }
                }
            }
            UnifiedCommand::Stop => {
                self.build_new_switch_vector("TELESCOPE_ABORT_MOTION", "ABORT", true)
            }
            UnifiedCommand::Park => self.build_new_switch_vector("TELESCOPE_PARK", "PARK", true),
            UnifiedCommand::Move { direction, speed } => {
                let rate = match speed {
                    Speed::Normalized(n) => *n,
                    Speed::Absolute(d) => *d / 10.0,
                };
                match direction {
                    MoveDirection::Up => self.build_new_switch_vector(
                        "TELESCOPE_MOTION_NS",
                        "MOTION_NORTH",
                        rate > 0.0,
                    ),
                    MoveDirection::Down => self.build_new_switch_vector(
                        "TELESCOPE_MOTION_NS",
                        "MOTION_SOUTH",
                        rate > 0.0,
                    ),
                    MoveDirection::Left => self.build_new_switch_vector(
                        "TELESCOPE_MOTION_WE",
                        "MOTION_WEST",
                        rate > 0.0,
                    ),
                    MoveDirection::Right => self.build_new_switch_vector(
                        "TELESCOPE_MOTION_WE",
                        "MOTION_EAST",
                        rate > 0.0,
                    ),
                    _ => Err(Error::NotSupported("Diagonal movement".into())),
                }
            }
            _ => Err(Error::NotSupported(format!("{:?}", cmd))),
        }
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        // Convert to string for parsing
        let text = match String::from_utf8(self.buffer.clone()) {
            Ok(s) => s,
            Err(_) => return Ok(DecodeResult::NeedMoreData),
        };

        // Try to find complete XML elements
        // INDI uses specific tag names: def*, set*, new*, get*, del*, message
        let element_patterns = [
            ("setNumberVector", "</setNumberVector>"),
            ("setSwitchVector", "</setSwitchVector>"),
            ("defNumberVector", "</defNumberVector>"),
            ("defSwitchVector", "</defSwitchVector>"),
            ("newNumberVector", "</newNumberVector>"),
            ("newSwitchVector", "</newSwitchVector>"),
            ("getProperties", "/>"),
            ("message", "</message>"),
        ];

        for (start_tag, end_pattern) in element_patterns {
            let start_pattern = format!("<{}", start_tag);
            if let Some(start_idx) = text.find(&start_pattern) {
                // Handle self-closing tags
                let search_from = start_idx + start_pattern.len();
                if let Some(relative_end) = text[search_from..].find(end_pattern) {
                    let end_idx = search_from + relative_end + end_pattern.len();
                    let xml_fragment = text[start_idx..end_idx].to_string();

                    // Remove processed data from buffer
                    self.buffer = self.buffer[end_idx..].to_vec();

                    return Self::parse_xml_element(&xml_fragment, start_tag);
                }
            }
        }

        Ok(DecodeResult::NeedMoreData)
    }

    fn encode_status(&self, status: &DeviceStatus) -> Result<Vec<u8>> {
        let state = match status.motion_state {
            MotionState::Moving => "Busy",
            MotionState::Idle => "Ok",
            MotionState::Error => "Alert",
            _ => "Idle",
        };

        if let Some(ref pos) = status.position {
            if pos.coord_system == CoordinateSystem::Equatorial {
                self.build_set_number_vector(
                    "EQUATORIAL_EOD_COORD",
                    &[("RA", pos.ra()), ("DEC", pos.dec())],
                    state,
                )
            } else {
                self.build_set_number_vector(
                    "HORIZONTAL_COORD",
                    &[("AZ", pos.azimuth()), ("ALT", pos.elevation())],
                    state,
                )
            }
        } else {
            Ok(Vec::new())
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            has_position_query: true,
            has_goto: true,
            has_continuous_move: true,
            has_stop: true,
            has_park: true,
            has_presets: false,
            has_speed_control: true,
            supports_equatorial: true,
            supports_altaz: true,
        }
    }
}
