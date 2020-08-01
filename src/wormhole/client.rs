use std::collections::VecDeque;
use std::fmt;
use std::time::Instant;

use actix::io::SinkWrite;
use actix::{Actor, AsyncContext, Context, Handler, StreamHandler};
use actix_codec::Framed;
use awc::{
    error::{WsClientError, WsProtocolError},
    ws::{Codec, Frame, Message},
    BoxedSocket,
};
use bytes::Bytes;
use futures::stream::{SplitSink, StreamExt};

use super::{Note, HEARTBEAT_INTERVAL, SERVER_TIMEOUT};

pub struct Wormhole {
    conn: actix::Addr<ServerConnection>,
}
impl Wormhole {
    pub async fn new() -> WormholeResult<Self> {
        use crate::client::{client, SERVER_URL};

        let (_, framed) = client()
            .ws(format!("{}/{}", *SERVER_URL, "wormhole"))
            .connect()
            .await?;

        let (sink, stream) = framed.split();
        Ok(Self {
            conn: ServerConnection::create(|ctx| {
                ServerConnection::add_stream(stream, ctx);
                ServerConnection::new(SinkWrite::new(sink, ctx))
            }),
        })
    }

    pub async fn try_note(&self) -> WormholeResult<Option<Note>> {
        self.conn.send(PopNote).await?
    }
}

#[derive(Debug)]
pub enum WormholeError {
    Mailbox(actix::MailboxError),
    WebSocket(WsProtocolError),
    Connection(String),
    Deserialization(serde_json::Error),
    Utf8(std::str::Utf8Error),
    ConnectionLost,
}

pub type WormholeResult<T> = Result<T, WormholeError>;

impl std::error::Error for WormholeError {}

impl fmt::Display for WormholeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use WormholeError::*;

        match self {
            Mailbox(e) => write!(f, "couldn't retrieve message from wormhole mailbox: {}", e),
            Connection(e) => write!(f, "couldn't connect to wormhole: {}", e),
            WebSocket(e) => write!(f, "error communicating with server through wormhole: {}", e),
            Deserialization(e) => write!(f, "error parsing server note from wormhole: {}", e),
            Utf8(e) => write!(f, "error parsing utf8 bytes from wormhole: {}", e),
            ConnectionLost => write!(f, "wormhole lost connection with server"),
        }
    }
}

impl From<WsProtocolError> for WormholeError {
    fn from(e: WsProtocolError) -> WormholeError {
        WormholeError::WebSocket(e)
    }
}

impl From<actix::MailboxError> for WormholeError {
    fn from(e: actix::MailboxError) -> WormholeError {
        WormholeError::Mailbox(e)
    }
}

impl From<WsClientError> for WormholeError {
    fn from(e: WsClientError) -> WormholeError {
        // normally I'd pass the error directly up the stack, but these mofos don't impl Send...
        WormholeError::Connection(format!("{}", e))
    }
}

impl From<serde_json::Error> for WormholeError {
    fn from(e: serde_json::Error) -> WormholeError {
        WormholeError::Deserialization(e)
    }
}

type ServerSink = SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>;
struct ServerConnection {
    to_server: ServerSink,
    last_heartbeat_from_server: Instant,
    notes: VecDeque<WormholeResult<Note>>,
    connected: bool,
}

#[derive(actix::Message)]
#[rtype(result = "WormholeResult<Option<Note>>")]
struct PopNote;

impl Actor for ServerConnection {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // start heartbeats, otherwise server will disconnect after 10 seconds
        self.heartbeat(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        log::error!("The wormhole is closing ...");
    }
}

impl ServerConnection {
    fn new(to_server: ServerSink) -> Self {
        Self {
            to_server,
            last_heartbeat_from_server: Instant::now(),
            notes: VecDeque::with_capacity(16),
            connected: false,
        }
    }

    fn heartbeat(&self, ctx: &mut Context<Self>) {
        ctx.run_later(HEARTBEAT_INTERVAL, |act, ctx| {
            act.to_server
                .write(Message::Ping(Bytes::from_static(b"")))
                .unwrap();
            act.heartbeat(ctx);

            // client should also check for a timeout here, similar to the server code
            let since_heartbeat = Instant::now().duration_since(act.last_heartbeat_from_server);
            if since_heartbeat > SERVER_TIMEOUT {
                act.connected = false;
            }
        });
    }
}

impl Handler<PopNote> for ServerConnection {
    type Result = WormholeResult<Option<Note>>;

    fn handle(&mut self, _: PopNote, _ctx: &mut Context<Self>) -> Self::Result {
        if self.connected {
            self.notes.pop_front().transpose()
        } else {
            Err(WormholeError::ConnectionLost)
        }
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for ServerConnection {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        self.notes.push_back(match msg {
            Ok(Frame::Text(s)) => std::str::from_utf8(&s)
                .map_err(|e| WormholeError::Utf8(e))
                .and_then(|s| serde_json::from_str(&s).map_err(|e| e.into())),
            Ok(Frame::Pong(_)) => {
                self.last_heartbeat_from_server = Instant::now();
                return;
            }
            Ok(_) => return,
            Err(e) => Err(e.into()),
        });
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("Wormhole connection established!");
    }

    fn finished(&mut self, _ctx: &mut Context<Self>) {
        log::error!("Server disconnected");
        self.connected = false;
    }
}

impl actix::io::WriteHandler<WsProtocolError> for ServerConnection {}
