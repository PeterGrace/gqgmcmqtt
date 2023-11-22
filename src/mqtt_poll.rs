use crate::consts::MQTT_POLL_INTERVAL_MILLIS;
use crate::ipc::{IPCMessage, InboundMessage};
use crate::mqtt_connection::MqttConnection;
use crate::payload::Payload;
use crate::errors::GQGMCMQTTError;
use rumqttc::{Event, Incoming, Outgoing, QoS};
use std::str;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::{sleep, timeout};

pub async fn mqtt_poll_loop(
    mqtt: MqttConnection,
    mut incoming_rx: tokio::sync::mpsc::Receiver<IPCMessage>,
    mut bcast_rx: tokio::sync::broadcast::Receiver<IPCMessage>,
    outgoing_tx: mpsc::Sender<IPCMessage>,
) -> Result<(), GQGMCMQTTError> {
    let task = tokio::spawn(async move {
        let mut conn = mqtt.event_loop;
        let mut dlq: Vec<u16> = vec![];
        loop {
            let notification = match conn.poll().await {
                Ok(event) => event,
                Err(e) => {
                    let msg = format!("Unable to poll mqtt: {e}");
                    panic!("{}", msg);
                }
            };

            match notification {
                Event::Incoming(i) => {
                    match i {
                        Incoming::Disconnect => {
                            // we should do something here.
                            error!("mqtt disconnect packet received.");
                            return;
                        }
                        Incoming::ConnAck(_ca) => {
                            info!("MQTT connection established.");
                        }
                        Incoming::PubAck(pa) => {
                            dlq.retain(|x| *x != pa.pkid);
                        }
                        Incoming::PingResp => {
                            trace!("Recv MQTT PONG");
                        }
                        Incoming::SubAck(_) => {}
                        Incoming::Publish(pr) => {}
                       _ => {
                            info!("mqtt incoming packet: {:#?}", i);
                        }
                    }
                }
                Event::Outgoing(o) => match o {
                    Outgoing::PingReq => {
                        trace!("Sent MQTT PING");
                    }
                    Outgoing::Publish(pb) => {
                        dlq.push(pb);
                    }
                    Outgoing::Subscribe(_) => {}
                    _ => {
                        info!("outgoing mqtt packet: {:#?}", o);
                    }
                },
            }
            if dlq.len() > 0 {
                trace!("DLQ is {}", dlq.len());
            }
        }
    });

    loop {
        if task.is_finished() {
            panic!("mqtt eventloop finished, this only happens in a connection error");
        }
        match bcast_rx.try_recv() {
            Ok(ipcm) => match ipcm {
                IPCMessage::Shutdown => {
                    info!("MQTT Received shutdown message, exiting thread.");
                    let _ = mqtt.client.disconnect().await;
                    return Err(GQGMCMQTTError::ExitingThread);
                }
                IPCMessage::Inbound(_) => {}
                IPCMessage::Outbound(_) => {}
                IPCMessage::PleaseReconnect(_, _) => {}
                IPCMessage::Error(_) => {}
            },
            Err(_) => {}
        }
        //region MQTT loop channel handling
        match incoming_rx.try_recv() {
            Ok(ipcm) => match ipcm {
                IPCMessage::Outbound(msg) => {
                    let payload = match serde_json::to_vec(&msg.payload) {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Payload couldn't be serialized to vec: {e}");
                            continue;
                        }
                    };
                    match timeout(
                        Duration::from_secs(3),
                        mqtt.client
                            .publish(msg.topic, QoS::AtLeastOnce, false, payload),
                    )
                    .await
                    {
                        Ok(result) => match result {
                            Ok(_) => {
                                if let Payload::Config(config) = msg.payload {
                                    let vals =
                                        config.unique_id.splitn(3, ".").collect::<Vec<&str>>();

                                };
                            }
                            Err(e) => {
                                error!("Couldn't send message: {e}");
                            }
                        },
                        Err(_e) => {
                            error!("Timeout trying to mqtt publish!")
                        }
                    }
                }
                IPCMessage::PleaseReconnect(_, _) => {
                    unreachable!();
                }
                IPCMessage::Inbound(_) => {
                    unreachable!();
                }
                IPCMessage::Shutdown => {
                    info!("MQTT Received shutdown message, exiting thread.");
                    let _ = mqtt.client.disconnect().await;
                    return Err(GQGMCMQTTError::ExitingThread);
                }
                _ => {}
            },
            Err(e) => match e {
                TryRecvError::Empty => {}
                TryRecvError::Disconnected => {
                    error!("We are disconnected!");
                }
            },
        }

        //endregion
        // trace!("mqtt tick");
        let _ = sleep(Duration::from_millis(MQTT_POLL_INTERVAL_MILLIS)).await;
    }
}
