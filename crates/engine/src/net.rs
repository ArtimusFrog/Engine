//! Netzwerk über UDP (renet + netcode).
//!
//! Die Engine verschickt nur Bytes über zwei Kanäle; was darin steht, legt das Spiel fest.
//! - [`Channel::Unreliable`]: schnell, darf verloren gehen (Positionen, Eingaben)
//! - [`Channel::Reliable`]: kommt garantiert und in Reihenfolge an (Ereignisse)

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime};

use renet::{ConnectionConfig, DefaultChannel, RenetClient, RenetServer};
use renet_netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication, ServerConfig,
};

pub type ClientId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Unreliable,
    Reliable,
}

impl From<Channel> for u8 {
    fn from(channel: Channel) -> u8 {
        match channel {
            Channel::Unreliable => DefaultChannel::Unreliable.into(),
            Channel::Reliable => DefaultChannel::ReliableOrdered.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerEvent {
    Connected(ClientId),
    Disconnected(ClientId, String),
}

fn now() -> Duration {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default()
}

pub struct NetServer {
    server: RenetServer,
    transport: NetcodeServerTransport,
}

impl NetServer {
    /// Wartet auf allen Netzwerkschnittstellen auf Verbindungen.
    ///
    /// `protocol_id` muss bei Server und Client gleich sein; verschiedene Spielversionen
    /// sollten verschiedene IDs nutzen, dann können sie sich gar nicht erst verbinden.
    pub fn listen(port: u16, protocol_id: u64, max_clients: usize) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        let address = socket.local_addr()?;
        let config = ServerConfig {
            current_time: now(),
            max_clients,
            protocol_id,
            public_addresses: vec![address],
            authentication: ServerAuthentication::Unsecure,
        };
        let transport = NetcodeServerTransport::new(config, socket)?;
        Ok(NetServer { server: RenetServer::new(ConnectionConfig::default()), transport })
    }

    pub fn port(&self) -> u16 {
        self.transport.addresses().first().map(|a| a.port()).unwrap_or(0)
    }

    /// Empfängt Pakete. Einmal pro Takt vor dem Lesen der Nachrichten aufrufen.
    pub fn receive(&mut self, dt: Duration) -> Vec<ServerEvent> {
        self.server.update(dt);
        if let Err(e) = self.transport.update(dt, &mut self.server) {
            log::warn!("Netzwerkfehler (Server): {e}");
        }
        let mut events = Vec::new();
        while let Some(event) = self.server.get_event() {
            events.push(match event {
                renet::ServerEvent::ClientConnected { client_id } => ServerEvent::Connected(client_id),
                renet::ServerEvent::ClientDisconnected { client_id, reason } => {
                    ServerEvent::Disconnected(client_id, reason.to_string())
                }
            });
        }
        events
    }

    pub fn clients(&self) -> Vec<ClientId> {
        self.server.clients_id()
    }

    pub fn message(&mut self, client: ClientId, channel: Channel) -> Option<Vec<u8>> {
        self.server.receive_message(client, channel).map(|b| b.to_vec())
    }

    pub fn send(&mut self, client: ClientId, channel: Channel, data: Vec<u8>) {
        self.server.send_message(client, channel, data);
    }

    pub fn broadcast(&mut self, channel: Channel, data: Vec<u8>) {
        self.server.broadcast_message(channel, data);
    }

    pub fn disconnect(&mut self, client: ClientId) {
        self.server.disconnect(client);
    }

    /// Round-Trip-Zeit zu einem Client in Sekunden.
    pub fn ping(&self, client: ClientId) -> f64 {
        self.server.rtt(client)
    }

    /// Verschickt alles, was seit dem letzten Aufruf gesendet wurde. Einmal pro Takt am Ende.
    pub fn flush(&mut self) {
        self.transport.send_packets(&mut self.server);
    }
}

impl Drop for NetServer {
    fn drop(&mut self) {
        self.transport.disconnect_all(&mut self.server);
    }
}

pub struct NetClient {
    client: RenetClient,
    transport: NetcodeClientTransport,
}

impl NetClient {
    /// Baut eine Verbindung zu `address` auf (z. B. `"127.0.0.1:7777"` oder `"spiel.example.com:7777"`).
    /// Die Verbindung steht erst, wenn [`is_connected`](Self::is_connected) `true` liefert.
    pub fn connect(address: &str, protocol_id: u64) -> std::io::Result<Self> {
        let server_addr: SocketAddr = address
            .to_socket_addrs()?
            .find(|a| a.is_ipv4())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("Adresse nicht gefunden: {address}")))?;
        let socket = UdpSocket::bind(("0.0.0.0", 0))?;
        let time = now();
        let authentication = ClientAuthentication::Unsecure {
            protocol_id,
            client_id: time.as_nanos() as u64 ^ std::process::id() as u64,
            server_addr,
            user_data: None,
        };
        let transport = NetcodeClientTransport::new(time, authentication, socket)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(NetClient { client: RenetClient::new(ConnectionConfig::default()), transport })
    }

    pub fn id(&self) -> ClientId {
        self.transport.client_id()
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// Grund der Trennung, falls die Verbindung weg ist.
    pub fn disconnected(&self) -> Option<String> {
        if let Some(reason) = self.client.disconnect_reason() {
            return Some(reason.to_string());
        }
        self.transport.disconnect_reason().map(|r| r.to_string())
    }

    pub fn receive(&mut self, dt: Duration) {
        self.client.update(dt);
        if let Err(e) = self.transport.update(dt, &mut self.client) {
            log::warn!("Netzwerkfehler (Client): {e}");
        }
    }

    pub fn message(&mut self, channel: Channel) -> Option<Vec<u8>> {
        self.client.receive_message(channel).map(|b| b.to_vec())
    }

    pub fn send(&mut self, channel: Channel, data: Vec<u8>) {
        self.client.send_message(channel, data);
    }

    /// Round-Trip-Zeit zum Server in Sekunden.
    pub fn ping(&self) -> f64 {
        self.client.rtt()
    }

    pub fn flush(&mut self) {
        if let Err(e) = self.transport.send_packets(&mut self.client) {
            log::warn!("Senden fehlgeschlagen: {e}");
        }
    }
}

impl Drop for NetClient {
    fn drop(&mut self) {
        self.transport.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_und_server_tauschen_nachrichten_aus() {
        let mut server = NetServer::listen(0, 42, 4).unwrap();
        let mut client = NetClient::connect(&format!("127.0.0.1:{}", server.port()), 42).unwrap();
        let dt = Duration::from_millis(16);

        let mut connected = None;
        for _ in 0..200 {
            client.receive(dt);
            client.flush();
            for event in server.receive(dt) {
                if let ServerEvent::Connected(id) = event {
                    connected = Some(id);
                }
            }
            server.flush();
            if connected.is_some() && client.is_connected() {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let id = connected.expect("Client hat sich nicht verbunden");
        assert_eq!(id, client.id());

        client.send(Channel::Reliable, b"hallo server".to_vec());
        server.send(id, Channel::Reliable, b"hallo client".to_vec());
        let (mut at_server, mut at_client) = (None, None);
        for _ in 0..200 {
            client.flush();
            server.flush();
            std::thread::sleep(Duration::from_millis(2));
            client.receive(dt);
            server.receive(dt);
            at_server = at_server.or_else(|| server.message(id, Channel::Reliable));
            at_client = at_client.or_else(|| client.message(Channel::Reliable));
            if at_server.is_some() && at_client.is_some() {
                break;
            }
        }
        assert_eq!(at_server.as_deref(), Some(&b"hallo server"[..]));
        assert_eq!(at_client.as_deref(), Some(&b"hallo client"[..]));
    }
}
