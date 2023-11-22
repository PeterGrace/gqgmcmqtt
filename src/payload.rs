use crate::consts::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use gqgmclib::GMC;
use crate::payload;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DeviceInfo {
    pub identifiers: Vec<String>,
    pub manufacturer: String,
    pub name: String,
    pub model: String,
    pub sw_version: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(untagged)]
pub enum PayloadValueType {
    Float(f32),
    Int(i64),
    String(String),
    Boolean(bool),
    #[default]
    None,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    Config(HAConfigPayload),
    CurrentState(StatePayload),
    #[default]
    None,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityCategory {
    Config,
    #[default]
    Diagnostic,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HAConfigPayload {
    pub name: String,
    pub device: DeviceInfo,
    pub unique_id: String,
    pub entity_id: String,
    pub state_topic: String,
    pub expires_after: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_category: Option<EntityCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_off: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "unit_of_measurement")]
    pub native_uom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_display_precision: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumed_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_picture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_state_attributes: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_entity_name: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_poll: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_press: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePayload {
    pub value: PayloadValueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub last_seen: DateTime<Utc>,
}

impl Default for StatePayload {
    fn default() -> Self {
        StatePayload {
            value: PayloadValueType::None,
            last_seen: Utc::now(),
            description: None,
            label: None,
            notes: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompoundPayload {
    pub(crate) config: HAConfigPayload,
    pub(crate) config_topic: String,
    pub(crate) state: StatePayload,
    pub(crate) state_topic: String,
}

pub async fn generate_payloads(gmc: &mut GMC) -> Vec<CompoundPayload> {
    let model = match &gmc.get_version().await {
        Ok(s) => s.clone(),
        Err(e) => {
            error!{"Can't get unit version: {e}"};
            return vec![]
        }
    };
    let serial = match &gmc.get_serial_number().await {
        Ok(s) => s.clone(),
        Err(e) => {
            error!("Can't get unit serial: {e}");
            return vec![]
        }
    };
    let device_info = payload::DeviceInfo {
        identifiers: vec![serial.clone()],
        manufacturer: "GQ Electronics".to_string(),
        name: "GQ Geiger Counter".to_string(),
        model: model.clone(),
        sw_version: "".to_string() };

    let cpm = match &gmc.get_cpm().await {
        Ok(cpm) => {
            if *cpm == 0 {
                return vec![];
            }
            *cpm
        },
        Err(e) => {
            error!{"Can't get cpm from device: {e}"};
            return vec![];
        }
    };

    let unit_name = format!("{model}-{serial}");

    let mut config_payload: HAConfigPayload = HAConfigPayload::default();
    let mut state_payload: StatePayload = StatePayload::default();

    let config_topic: String = format!("homeassistant/sensor/{serial}/geiger_counter_cpm/config");
    let state_topic = format!("gqgmcmqtt/{serial}/geiger_counter_cpm");
    config_payload.state_topic = state_topic.clone();

    config_payload.name = unit_name.clone();
    config_payload.device_class = None;
    config_payload.state_class = Some("measurement".to_string());
    config_payload.expires_after = 300;
    config_payload.value_template = Some("{{ value_json.value }}".to_string());
    config_payload.unique_id = unit_name.clone();
    config_payload.entity_id = format!("sensor.{serial}_geiger_tube_cpm");
    config_payload.suggested_display_precision = Some(0);
    config_payload.native_uom = Some("cpm".to_string());
    config_payload.device = device_info;
    config_payload.icon = Some("mdi:radioactive".to_string());


    state_payload.value = PayloadValueType::Int(cpm as i64);



    let resp = CompoundPayload {
        config: config_payload,
        state: state_payload,
        config_topic,
        state_topic,
    };
    vec![resp]
}
