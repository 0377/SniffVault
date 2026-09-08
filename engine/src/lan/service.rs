use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::EngineError;
use crate::lan::{
    begin_pairing, discover_receivers, register_receiver, select_for_peer, sign_request,
    unregister, CastEvent, CastMetadata, CastPlayRequest, LanPeer, PairingSession, ReceiverHttp,
    ReceiverRegistration, ReceiverState, SenderHttp, SenderState, StreamTokenStore, TrustStore,
    TrustedPeer,
};
use crate::types::LibraryEpisode;

const DISCOVER_CACHE_TTL: Duration = Duration::from_secs(5);
const DISCOVER_TIMEOUT: Duration = Duration::from_secs(3);
const CAST_EVENT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

pub type GetEpisodeFn = Arc<dyn Fn(&str) -> Option<LibraryEpisode> + Send + Sync>;

#[derive(Debug, Clone, Default)]
pub struct LanTestConfig {
    pub advertise_ip: Option<String>,
}

struct ActiveCastSession {
    session_id: String,
    stream_token: String,
    peer_device_id: String,
    peer_host: String,
    peer_port: u16,
}

pub struct LanService {
    trust_store: Arc<Mutex<TrustStore>>,
    runtime: Option<Runtime>,
    test_config: LanTestConfig,
    device_id: String,
    device_name: String,
    is_receiver: bool,
    sender: Option<SenderHttp>,
    sender_port: Option<u16>,
    receiver: Option<ReceiverHttp>,
    receiver_port: Option<u16>,
    mdns: Option<ReceiverRegistration>,
    stream_tokens: Arc<Mutex<StreamTokenStore>>,
    pairing_session: Arc<Mutex<Option<PairingSession>>>,
    cast_rx: Option<mpsc::Receiver<CastEvent>>,
    peers_cache: Option<(Instant, Vec<LanPeer>)>,
    active_cast: Option<ActiveCastSession>,
    media_dir: PathBuf,
    get_episode: Option<GetEpisodeFn>,
}

impl LanService {
    pub fn open(trust_db_path: &Path) -> Result<Self, EngineError> {
        let runtime = Runtime::new()?;
        let trust_store = Arc::new(Mutex::new(TrustStore::open(trust_db_path)?));
        Ok(Self {
            trust_store,
            runtime: Some(runtime),
            test_config: LanTestConfig::default(),
            device_id: String::new(),
            device_name: String::new(),
            is_receiver: false,
            sender: None,
            sender_port: None,
            receiver: None,
            receiver_port: None,
            mdns: None,
            stream_tokens: Arc::new(Mutex::new(StreamTokenStore::new())),
            pairing_session: Arc::new(Mutex::new(None)),
            cast_rx: None,
            peers_cache: None,
            active_cast: None,
            media_dir: PathBuf::new(),
            get_episode: None,
        })
    }

    pub fn set_test_config(&mut self, config: LanTestConfig) {
        self.test_config = config;
    }

    pub fn set_device_identity(&mut self, device_id: &str, device_name: &str) {
        self.device_id = device_id.to_string();
        self.device_name = device_name.to_string();
    }

    pub fn lan_http_port(&self) -> Option<u16> {
        if self.is_receiver {
            self.receiver_port
        } else {
            self.sender_port
        }
    }

    pub fn list_trusted_peers(&self) -> Result<Vec<TrustedPeer>, EngineError> {
        self.trust_store
            .lock()
            .map_err(|_| lock_err())?
            .list_peers()
    }

    pub fn remove_trusted_peer(&self, peer_device_id: &str) -> Result<bool, EngineError> {
        self.trust_store
            .lock()
            .map_err(|_| lock_err())?
            .remove_peer(peer_device_id)
    }

    pub fn begin_pairing(&mut self) -> Result<String, EngineError> {
        let session = begin_pairing(now_ms());
        let pin = session.pin.clone();
        *self.pairing_session.lock().map_err(|_| lock_err())? = Some(session);
        Ok(pin)
    }

    pub fn pairing_pin(&self) -> Result<Option<String>, EngineError> {
        let guard = self.pairing_session.lock().map_err(|_| lock_err())?;
        Ok(guard.as_ref().map(|session| session.pin.clone()))
    }

    pub fn start(
        &mut self,
        is_receiver: bool,
        device_id: &str,
        device_name: &str,
        media_dir: &Path,
        get_episode: Option<GetEpisodeFn>,
    ) -> Result<(), EngineError> {
        if self.is_running() {
            if self.is_receiver == is_receiver {
                return Ok(());
            }
            self.stop()?;
        }

        self.device_id = device_id.to_string();
        self.device_name = device_name.to_string();
        self.is_receiver = is_receiver;
        self.media_dir = media_dir.to_path_buf();
        self.get_episode = get_episode;

        let bind: SocketAddr = "0.0.0.0:0".parse().expect("valid bind address");
        if is_receiver {
            self.start_receiver(bind)?;
        } else {
            self.start_sender(bind)?;
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), EngineError> {
        self.stop_cast_internal(false)?;
        if let Some(reg) = self.mdns.take() {
            unregister(reg)?;
        }
        if let Some(server) = self.sender.take() {
            if let Some(runtime) = self.runtime.as_ref() {
                runtime.block_on(server.stop());
            }
        }
        if let Some(server) = self.receiver.take() {
            if let Some(runtime) = self.runtime.as_ref() {
                runtime.block_on(server.stop());
            }
        }
        self.sender_port = None;
        self.receiver_port = None;
        self.cast_rx = None;
        self.is_receiver = false;
        Ok(())
    }

    pub fn discover_peers(&mut self) -> Result<Vec<LanPeer>, EngineError> {
        if let Some((cached_at, peers)) = &self.peers_cache {
            if cached_at.elapsed() < DISCOVER_CACHE_TTL {
                return Ok(peers.clone());
            }
        }

        let mut peers = discover_receivers(DISCOVER_TIMEOUT)?;
        let trusted_ids: HashMap<String, TrustedPeer> = self
            .list_trusted_peers()?
            .into_iter()
            .map(|peer| (peer.peer_device_id.clone(), peer))
            .collect();

        for peer in &mut peers {
            peer.is_trusted = trusted_ids.contains_key(&peer.device_id);
            if let Some(trusted) = trusted_ids.get(&peer.device_id) {
                peer.host = trusted.peer_host.clone();
                peer.port = trusted.peer_port;
            }
        }

        self.peers_cache = Some((Instant::now(), peers.clone()));
        Ok(peers)
    }

    pub fn pair_peer(
        &mut self,
        host: &str,
        port: u16,
        pin: &str,
        sender_device_id: &str,
        sender_device_name: &str,
    ) -> Result<(), EngineError> {
        if !self.is_sender_running() {
            return Err(EngineError::InvalidArg(
                "sender LAN service is not running".into(),
            ));
        }

        self.runtime.as_ref().expect("lan runtime").block_on(async {
            let client = reqwest::Client::new();
            let health: serde_json::Value = client
                .get(format!("http://{host}:{port}/health"))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let receiver_device_id = health
                .get("device_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| EngineError::Message("pair health missing device_id".into()))?
                .to_string();

            let pair_body = serde_json::json!({
                "device_id": sender_device_id,
                "device_name": sender_device_name,
                "pin": pin,
            });
            let pair_resp: serde_json::Value = client
                .post(format!("http://{host}:{port}/v1/pair"))
                .json(&pair_body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let secret_hex = pair_resp
                .get("session_secret")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    EngineError::Message("pair response missing session_secret".into())
                })?;
            let session_secret = decode_session_secret(secret_hex)?;

            self.trust_store
                .lock()
                .map_err(|_| lock_err())?
                .upsert_peer(&TrustedPeer {
                    peer_device_id: receiver_device_id,
                    peer_name: "TV".into(),
                    peer_host: host.to_string(),
                    peer_port: port,
                    paired_at_ms: now_ms(),
                    session_secret: Some(session_secret),
                })?;
            self.peers_cache = None;
            Ok(())
        })
    }

    pub fn cast_episode(
        &mut self,
        episode_id: &str,
        peer_device_id: &str,
        metadata: CastMetadata,
        sender_device_id: &str,
        sender_device_name: &str,
    ) -> Result<(), EngineError> {
        if !self.is_sender_running() {
            return Err(EngineError::InvalidArg(
                "sender LAN service is not running".into(),
            ));
        }

        let peer = self
            .list_trusted_peers()?
            .into_iter()
            .find(|peer| peer.peer_device_id == peer_device_id)
            .ok_or_else(|| EngineError::InvalidArg("peer is not trusted".into()))?;
        let session_secret = peer
            .session_secret
            .ok_or_else(|| EngineError::InvalidArg("peer missing session secret".into()))?;

        let token = {
            let mut store = self.stream_tokens.lock().map_err(|_| lock_err())?;
            store.issue(episode_id, now_secs())
        };

        let sender_port = self
            .sender_port
            .ok_or_else(|| EngineError::Message("sender port unavailable".into()))?;
        let advertise_ip = self.advertise_ip(&peer.peer_host);
        let stream_url = format!("http://{advertise_ip}:{sender_port}/v1/stream/{token}");
        let session_id = Uuid::new_v4().to_string();
        let request = CastPlayRequest {
            session_id: session_id.clone(),
            sender_device_id: sender_device_id.to_string(),
            sender_name: sender_device_name.to_string(),
            metadata,
            stream_url,
        };

        let body = serde_json::to_vec(&request)?;
        let timestamp = now_secs();
        let signature = sign_request(
            &session_secret,
            sender_device_id,
            timestamp,
            "POST",
            "/v1/cast/play",
            &body,
        );

        self.runtime
            .as_ref()
            .expect("lan runtime")
            .block_on(async {
                let client = reqwest::Client::new();
                client
                    .post(format!(
                        "http://{}:{}/v1/cast/play",
                        peer.peer_host, peer.peer_port
                    ))
                    .header("X-SniffVault-Device-Id", sender_device_id)
                    .header("X-SniffVault-Timestamp", timestamp.to_string())
                    .header("X-SniffVault-Signature", signature)
                    .body(body)
                    .send()
                    .await?
                    .error_for_status()?;
                Ok::<(), EngineError>(())
            })?;

        self.trust_store
            .lock()
            .map_err(|_| lock_err())?
            .upsert_peer(&TrustedPeer {
                peer_device_id: peer.peer_device_id.clone(),
                peer_name: peer.peer_name.clone(),
                peer_host: peer.peer_host.clone(),
                peer_port: peer.peer_port,
                paired_at_ms: peer.paired_at_ms,
                session_secret: peer.session_secret,
            })?;

        self.active_cast = Some(ActiveCastSession {
            session_id,
            stream_token: token,
            peer_device_id: peer.peer_device_id,
            peer_host: peer.peer_host,
            peer_port: peer.peer_port,
        });
        Ok(())
    }

    pub fn stop_cast(&mut self) -> Result<(), EngineError> {
        self.stop_cast_internal(true)
    }

    pub fn take_cast_event_receiver(&mut self) -> Option<mpsc::Receiver<CastEvent>> {
        self.cast_rx.take()
    }

    pub fn drain_cast_event(&mut self) -> Result<CastEvent, EngineError> {
        let rx = self
            .cast_rx
            .as_mut()
            .ok_or_else(|| EngineError::InvalidArg("cast event receiver not available".into()))?;
        self.runtime
            .as_ref()
            .expect("lan runtime")
            .block_on(async { tokio::time::timeout(CAST_EVENT_DRAIN_TIMEOUT, rx.recv()).await })
            .map_err(|_| EngineError::Message("cast event drain timed out".into()))?
            .ok_or_else(|| EngineError::Message("cast event channel closed".into()))
    }

    fn start_sender(&mut self, bind: SocketAddr) -> Result<(), EngineError> {
        let get_episode = self.get_episode.clone().ok_or_else(|| {
            EngineError::InvalidArg("sender episode lookup not configured".into())
        })?;
        let state = SenderState {
            device_id: self.device_id.clone(),
            stream_token_store: Arc::clone(&self.stream_tokens),
            media_dir: self.media_dir.clone(),
            get_episode,
        };
        let (server, port) = self
            .runtime
            .as_ref()
            .expect("lan runtime")
            .block_on(SenderHttp::start(bind, state))?;
        self.sender = Some(server);
        self.sender_port = Some(port);
        Ok(())
    }

    fn start_receiver(&mut self, bind: SocketAddr) -> Result<(), EngineError> {
        let (event_tx, event_rx) = mpsc::channel(16);
        let state = ReceiverState {
            device_id: self.device_id.clone(),
            trust_store: Arc::clone(&self.trust_store),
            pairing_session: Arc::clone(&self.pairing_session),
            event_tx,
        };
        let (server, port) = self
            .runtime
            .as_ref()
            .expect("lan runtime")
            .block_on(ReceiverHttp::start(bind, state))?;
        self.receiver = Some(server);
        self.receiver_port = Some(port);
        self.cast_rx = Some(event_rx);

        if self.test_config.advertise_ip.is_none() {
            let host = crate::lan::enumerate_local_ipv4()
                .into_iter()
                .find(|ip| ip != "127.0.0.1")
                .unwrap_or_else(|| "127.0.0.1".to_string());
            self.mdns = Some(register_receiver(
                &self.device_id,
                &self.device_name,
                &host,
                port,
            )?);
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.sender.is_some() || self.receiver.is_some()
    }

    fn is_sender_running(&self) -> bool {
        self.sender.is_some()
    }

    fn advertise_ip(&self, peer_host: &str) -> String {
        if let Some(ip) = &self.test_config.advertise_ip {
            return ip.clone();
        }
        let local_ips = crate::lan::enumerate_local_ipv4();
        let refs: Vec<&str> = local_ips.iter().map(String::as_str).collect();
        select_for_peer(peer_host, &refs)
    }

    fn stop_cast_internal(&mut self, notify_peer: bool) -> Result<(), EngineError> {
        let Some(active) = self.active_cast.take() else {
            return Ok(());
        };

        {
            let mut store = self.stream_tokens.lock().map_err(|_| lock_err())?;
            store.revoke(&active.stream_token);
        }

        if notify_peer {
            if let Ok(peers) = self.list_trusted_peers() {
                if let Some(peer) = peers
                    .into_iter()
                    .find(|peer| peer.peer_device_id == active.peer_device_id)
                {
                    if let Some(secret) = peer.session_secret {
                        let body = serde_json::to_vec(&serde_json::json!({
                            "session_id": active.session_id,
                        }))?;
                        let timestamp = now_secs();
                        let signature = sign_request(
                            &secret,
                            &self.device_id,
                            timestamp,
                            "POST",
                            "/v1/cast/stop",
                            &body,
                        );
                        self.runtime
                            .as_ref()
                            .expect("lan runtime")
                            .block_on(async {
                                let client = reqwest::Client::new();
                                let _ = client
                                    .post(format!(
                                        "http://{}:{}/v1/cast/stop",
                                        active.peer_host, active.peer_port
                                    ))
                                    .header("X-SniffVault-Device-Id", &self.device_id)
                                    .header("X-SniffVault-Timestamp", timestamp.to_string())
                                    .header("X-SniffVault-Signature", signature)
                                    .body(body)
                                    .send()
                                    .await;
                                Ok::<(), EngineError>(())
                            })?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for LanService {
    fn drop(&mut self) {
        let _ = self.stop();
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

fn decode_session_secret(hex_str: &str) -> Result<[u8; 32], EngineError> {
    let bytes = hex::decode(hex_str)
        .map_err(|_| EngineError::InvalidArg("invalid session_secret hex".into()))?;
    if bytes.len() != 32 {
        return Err(EngineError::InvalidArg(
            "session_secret must be 32 bytes".into(),
        ));
    }
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&bytes);
    Ok(secret)
}

fn lock_err() -> EngineError {
    EngineError::Message("lan service lock poisoned".into())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
