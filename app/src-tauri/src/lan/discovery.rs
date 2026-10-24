//! Finding the other devices: mDNS service `_musicapp._tcp` with TXT `id` and
//! `name`. Peers come and go as the daemon reports them; every change is pushed as
//! [`LanPeers`](super::LanPeers).

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use super::{Lan, LanPeers, Peer};
use crate::error::{AppError, AppResult};

const SERVICE_TYPE: &str = "_musicapp._tcp.local.";

fn net_error(e: mdns_sd::Error) -> AppError {
    AppError::Network(format!("découverte des appareils impossible : {e}"))
}

impl Lan {
    /// Starts the HTTP server, advertises this device and browses for the others.
    pub async fn start(&self) -> AppResult<()> {
        let port = self.serve().await?;
        let daemon = ServiceDaemon::new().map_err(net_error)?;
        let events = daemon.browse(SERVICE_TYPE).map_err(net_error)?;
        {
            let mut state = self.state();
            state.port = port;
            state.daemon = Some(daemon);
        }
        self.advertise();

        let lan = self.clone();
        tauri::async_runtime::spawn(async move {
            while let Ok(event) = events.recv_async().await {
                lan.on_event(event);
            }
        });
        Ok(())
    }

    /// (Re)announces this device with its current name.
    pub(super) fn advertise(&self) {
        let (name, port) = {
            let state = self.state();
            (state.name.clone(), state.port)
        };
        let state = self.state();
        let Some(daemon) = &state.daemon else {
            return;
        };
        let id = &self.inner.device_id;
        let props = [("id", id.as_str()), ("name", name.as_str())];
        let host = format!("musicapp-{id}.local.");
        // Registering again under the same name updates the announcement.
        let result = ServiceInfo::new(SERVICE_TYPE, id, &host, "", port, &props[..])
            .map(ServiceInfo::enable_addr_auto)
            .and_then(|info| daemon.register(info));
        if let Err(e) = result {
            tracing::warn!("cannot advertise on the network: {e}");
        }
    }

    pub fn shutdown(&self) {
        if let Some(daemon) = self.state().daemon.take() {
            let _ = daemon.shutdown();
        }
    }

    fn on_event(&self, event: ServiceEvent) {
        let changed = match event {
            ServiceEvent::ServiceResolved(info) => {
                let id = info.get_property_val_str("id").unwrap_or_default();
                let addr = info.get_addresses_v4().into_iter().min().copied();
                match (id, addr) {
                    (id, Some(addr)) if !id.is_empty() && id != self.inner.device_id => {
                        let name = info
                            .get_property_val_str("name")
                            .and_then(super::clean_name)
                            .unwrap_or_else(|| "Appareil inconnu".into());
                        let peer = Peer {
                            id: id.to_owned(),
                            name,
                            addr: addr.to_string(),
                            port: info.get_port(),
                        };
                        self.state()
                            .peers
                            .insert(info.get_fullname().to_owned(), peer.clone())
                            != Some(peer)
                    }
                    _ => false,
                }
            }
            ServiceEvent::ServiceRemoved(_, fullname) => {
                self.state().peers.remove(&fullname).is_some()
            }
            _ => false,
        };
        if changed {
            let peers = self.state().peers.values().cloned().collect();
            self.emit(LanPeers { peers });
        }
    }
}
