//! IoT sensor adapter — MQTT/NATS streaming, CamoFox JSON, NMEA 0183 GPS.
#![allow(missing_docs)]

pub mod iot_adapter;
#[cfg(feature = "mqtt")]
pub mod iot_mqtt;
pub mod iot_tools;
