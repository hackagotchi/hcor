use std::collections::VecDeque;
use std::fmt;
use std::time::Instant;

use actix::{
    io::SinkWrite, Actor, Addr, AsyncContext, Context, Handler, ResponseFuture, StreamHandler,
};
use actix_codec::Framed;
use awc::{
    error::{WsClientError, WsProtocolError},
    ws::{Codec, Frame, Message},
    BoxedSocket,
};
use bytes::Bytes;
use futures::{
    channel::oneshot,
    stream::{SplitSink, StreamExt},
};
use log::*;

use super::{
    Ask, AskMessage, AskedNote, EstablishWormholeRequest, Note, HEARTBEAT_INTERVAL, SERVER_TIMEOUT,
};
use crate::{IdentifiesUser, UserId};

type ConnAddr = Addr<ServerConnection>;

#[cfg(feature = "simultaneous_systems")]
use dashmap::DashMap;

#[cfg(feature = "simultaneous_systems")]
lazy_static::lazy_static! {
    static ref CONNS: DashMap<usize, ConnAddr> = DashMap::new();
}
#[cfg(not(feature = "simultaneous_systems"))]
lazy_static::lazy_static! {
    static ref CONN: ConnAddr = ServerConnection::start_default();
}

#[cfg(feature = "simultaneous_systems")]
fn get_conn() -> ConnAddr {
    CONNS
        .get(&actix::System::current().id())
        .expect("no server connection for this actix::System")
        .value()
        .clone()
}

#[cfg(not(feature = "simultaneous_systems"))]
fn get_conn() -> &'static ConnAddr {
    *CONN
}

pub async fn connect(iu: impl IdentifiesUser) -> WormholeResult<()> {
    #[cfg(feature = "simultaneous_systems")]
    {
        CONNS.insert(
            actix::System::current().id(),
            ServerConnection::start_default(),
        );
        debug!("connections count +1, now: {}", CONNS.len());
    }

    get_conn().send(Connect(iu.user_id())).await??;

    Ok(())
}

pub async fn try_note() -> WormholeResult<Option<Note>> {
    get_conn().send(PopNote).await?
}

pub async fn ask(ask: Ask) -> WormholeResult<usize> {
    get_conn().send(SendAsk(ask)).await?
}

#[cfg(feature = "simultaneous_systems")]
pub async fn disconnect() -> WormholeResult<()> {
    let (_, addr) = CONNS
        .remove(&actix::System::current().id())
        .ok_or(WormholeError::AlreadyDisconnected)?;
    addr.send(Disconnect).await?
}

#[cfg(not(feature = "simultaneous_systems"))]
pub async fn disconnect() -> WormholeResult<()> {
    get_conn().send(Disconnect).await?
}

#[derive(Copy, Clone)]
pub enum ContinueBehavior {
    Pass,
    Consume,
}

/// Used to implement `until` and `until_map`, as well as their greedy counterparts.
/// Return `Ok(T)` from the handler function to yield `T` from the future this function returns.
/// Note that this removes your handler.
///
/// Return `Err(ContinueBehavior::Pass)` to non-greedily ignore the event, and return
/// `Err(ContinueBehavior::Consume)` to prevent handlers registered after this one from seeing it.
///
/// Note that input functions are not exposed to events sitting in the wormhole's queue, waiting to
/// be retrieved using `try_note`. All events which are never consumed by a handler will end up in
/// this queue.
pub async fn register_note_handler<
    T: fmt::Debug + Send + 'static,
    F: FnMut(&Note) -> Result<T, ContinueBehavior> + Send + 'static,
>(
    handler_fn: F,
) -> WormholeResult<T> {
    let (uh, rx) = NoteHandler::new(handler_fn);

    get_conn().send(RegisterNoteHandler(Box::new(uh))).await?;

    rx.await.map_err(|e| e.into())
}

/// Calls the input function with every Note received from the wormhole,
/// yielding the first Note which the input function returns true for.
///
/// Notes which the function returns false for are simply ignored. If you want them to be consumed
/// such that handlers registered later than this one will not receive the note, see `until_greedy`.
pub async fn until<F: FnMut(&Note) -> bool + Send + 'static>(mut f: F) -> WormholeResult<Note> {
    register_note_handler(move |n| {
        if f(n) {
            Ok(n.clone())
        } else {
            Err(ContinueBehavior::Pass)
        }
    })
    .await
}

/// Calls the input function on every AskedNote received from the wormhole whose `ask_id` matches the
/// provided one. The first time the input function returns `true` when provided with such a
/// note, the future this function returns yields WormholeResult<AskedNote>.
pub async fn until_ask_id<F: FnMut(&AskedNote) -> bool + Send + 'static>(
    ask_id: usize,
    mut f: F,
) -> WormholeResult<AskedNote> {
    register_note_handler(move |n| match n {
        Note::Asked { ask_id: id, note } if ask_id == *id && f(&note) => Ok(note.clone()),
        _ => Err(ContinueBehavior::Pass),
    })
    .await
}

/// Calls the input function with every Note received from the wormhole,
/// yielding the first Note which the input function returns true for.
///
/// Notes which the function returns false for are consumed, so that handlers registered
/// later than this one will not receive them. (However, handlers registered before this one will.)
/// If you would like these notes to simply be ignored instead of consumed, see `until`.
pub async fn until_greedy<F: FnMut(&Note) -> bool + Send + 'static>(
    mut f: F,
) -> WormholeResult<Note> {
    register_note_handler(move |n| {
        if f(n) {
            Ok(n.clone())
        } else {
            Err(ContinueBehavior::Consume)
        }
    })
    .await
}

/// Calls the input function on every AskedNote received from the wormhole whose `ask_id` matches the
/// provided one. The first time the input function returns Some(T) when provided with such a
/// note, the future this function returns yields WormholeResult<T>.
///
/// Each note for which the input function returns None is simply ignored, so that other handlers
/// may process it. If this is not desired, see `until_map_greedy`.
pub async fn until_ask_id_map<
    T: fmt::Debug + Send + 'static,
    F: FnMut(AskedNote) -> Option<T> + Send + 'static,
>(
    ask_id: usize,
    mut f: F,
) -> WormholeResult<T> {
    register_note_handler(move |n| match n {
        Note::Asked { ask_id: id, note } if ask_id == *id => {
            f(note.clone()).ok_or(ContinueBehavior::Pass)
        }
        _ => Err(ContinueBehavior::Pass),
    })
    .await
}

/// Calls the input function on every AskedNote received from the wormhole whose `ask_id` matches the
/// provided one. The first time the input function returns Some(T) when provided with such a
/// note, the future this function returns yields WormholeResult<T>.
///
/// Each note for which the input function returns None is consumed, so that handlers registered
/// after this one will be deprived of it. If this behavior is not desired, see `until_map`, which
/// will simply ignore these events.
pub async fn until_ask_id_map_greedy<
    T: fmt::Debug + Send + 'static,
    F: FnMut(AskedNote) -> Option<T> + Send + 'static,
>(
    ask_id: usize,
    mut f: F,
) -> WormholeResult<T> {
    register_note_handler(move |n| match n {
        Note::Asked { ask_id: id, note } if ask_id == *id => {
            f(note.clone()).ok_or(ContinueBehavior::Consume)
        }
        _ => Err(ContinueBehavior::Consume),
    })
    .await
}

/// Calls the input function with every Note received from the wormhole,
/// yielding the first Note which the input function returns Some(T) for.
/// The output of this future will be whatever T is supplied.
///
/// Notes which the function returns None for are simply ignored. If you want them to be consumed
/// such that handlers registered later than this one will not receive the note, see `until_map_greedy`.
pub async fn until_map<
    T: fmt::Debug + Send + 'static,
    F: FnMut(Note) -> Option<T> + Send + 'static,
>(
    mut f: F,
) -> WormholeResult<T> {
    register_note_handler(move |n| f(n.clone()).ok_or(ContinueBehavior::Consume)).await
}

trait ContinueHandling: Send {
    fn continue_handling(&mut self, note: &Note) -> Option<ContinueBehavior>;
}

struct NoteHandler<T, F> {
    tx: Option<oneshot::Sender<T>>,
    handler_fn: F,
}
impl<T, F> NoteHandler<T, F> {
    fn new(handler_fn: F) -> (Self, oneshot::Receiver<T>) {
        let (tx, rx) = oneshot::channel();
        (
            Self {
                tx: Some(tx),
                handler_fn,
            },
            rx,
        )
    }
}

impl<T: fmt::Debug + Send + 'static, F: FnMut(&Note) -> Result<T, ContinueBehavior> + Send>
    ContinueHandling for NoteHandler<T, F>
{
    fn continue_handling(&mut self, n: &Note) -> Option<ContinueBehavior> {
        match (self.handler_fn)(n) {
            Ok(t) => {
                if let Some(tx) = self.tx.take() {
                    tx.send(t).unwrap()
                }

                None
            }
            Err(cb) => Some(cb),
        }
    }
}

#[derive(Debug)]
pub enum WormholeError {
    Mailbox(actix::MailboxError),
    WebSocket(WsProtocolError),
    Connection(String),
    Serde(serde_json::Error),
    Utf8(std::str::Utf8Error),
    ConnectionLost,
    NeverConnected,
    AlreadyDisconnected,
    NoteHandlerCanceled,
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
            Serde(e) => write!(f, "error parsing or formatting from or for wormhole: {}", e),
            Utf8(e) => write!(f, "error parsing utf8 bytes from wormhole: {}", e),
            AlreadyDisconnected => write!(
                f,
                "disconnect has been called again following disconnecting"
            ),
            ConnectionLost => write!(f, "wormhole lost connection with server"),
            NeverConnected => write!(f, "wormhole connection never established"),
            NoteHandlerCanceled => {
                write!(f, "receiver for response from note handler was canceled")
            }
        }
    }
}

impl From<WsProtocolError> for WormholeError {
    fn from(e: WsProtocolError) -> WormholeError {
        WormholeError::WebSocket(e)
    }
}

impl From<oneshot::Canceled> for WormholeError {
    fn from(_: oneshot::Canceled) -> WormholeError {
        // there's no point in logging that error, it just says "oneshot canceled" no matter what.
        WormholeError::NoteHandlerCanceled
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
        WormholeError::Serde(e)
    }
}

struct ServerConnection {
    notes: VecDeque<WormholeResult<Note>>,
    note_handlers: Vec<Box<dyn ContinueHandling>>,
    state: State,
    asks_sent: usize,
    user: Option<UserId>,
}

impl Default for ServerConnection {
    fn default() -> Self {
        Self {
            notes: VecDeque::with_capacity(16),
            note_handlers: vec![],
            state: State::NotConnected,
            asks_sent: 0,
            user: None,
        }
    }
}

impl Actor for ServerConnection {
    type Context = Context<Self>;

    /*
    fn stopping(&mut self, _: &mut Context<Self>) -> actix::Running {
        info!("averting actor stop");
        actix::Running::Continue
    }*/

    fn stopped(&mut self, _: &mut Context<Self>) {
        error!("The wormhole is closing ...");
    }
}

impl ServerConnection {
    fn heartbeat(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, _ctx| {
            let disconnect = match &mut act.state {
                State::Connected(to_server, hb) => {
                    to_server
                        .write(Message::Ping(Bytes::from_static(b"")))
                        .unwrap();

                    let since_heartbeat = Instant::now().duration_since(*hb);
                    if since_heartbeat > SERVER_TIMEOUT {
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if disconnect {
                act.state = State::ConnectionLost;
            }
        });
    }
}

type ServerSink = SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>;
enum State {
    NotConnected,
    WebsocketsConnected,
    Connected(ServerSink, Instant),
    ConnectionLost,
}
impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use State::*;

        match self {
            NotConnected => write!(f, "NotConnected"),
            WebsocketsConnected => write!(f, "WebsocketsConnected"),
            Connected(_, _) => write!(f, "Connected"),
            ConnectionLost => write!(f, "ConnectionLost"),
        }
    }
}
impl Default for State {
    fn default() -> Self {
        State::NotConnected
    }
}
impl State {
    fn update_heartbeat(&mut self) {
        use State::*;

        *self = match std::mem::take(self) {
            Connected(s, _) => Connected(s, Instant::now()),
            o => o,
        };
    }
}

#[derive(actix::Message)]
#[rtype(result = "WormholeResult<Option<Note>>")]
struct PopNote;

impl Handler<PopNote> for ServerConnection {
    type Result = WormholeResult<Option<Note>>;

    fn handle(&mut self, _: PopNote, _ctx: &mut Context<Self>) -> Self::Result {
        use State::*;

        match self.state {
            Connected(_, _) => self.notes.pop_front().transpose(),
            WebsocketsConnected => Ok(None),
            ConnectionLost => Err(WormholeError::ConnectionLost),
            NotConnected => Err(WormholeError::NeverConnected),
        }
    }
}

#[derive(actix::Message)]
#[rtype(result = "()")]
struct RegisterNoteHandler(Box<dyn ContinueHandling>);

impl Handler<RegisterNoteHandler> for ServerConnection {
    type Result = ();

    fn handle(
        &mut self,
        RegisterNoteHandler(f): RegisterNoteHandler,
        _: &mut Context<Self>,
    ) -> Self::Result {
        self.note_handlers.push(f);
    }
}

#[derive(actix::Message)]
#[rtype(result = "WormholeResult<()>")]
struct Disconnect;

impl Handler<Disconnect> for ServerConnection {
    type Result = WormholeResult<()>;

    fn handle(&mut self, _: Disconnect, _: &mut Context<Self>) -> Self::Result {
        match &mut self.state {
            State::Connected(s, _) => {
                s.write(Message::Close(Some(awc::ws::CloseReason {
                    code: awc::ws::CloseCode::Normal,
                    description: Some("cya nerd".to_string()),
                })))?;
                self.state = State::ConnectionLost;
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(actix::Message)]
#[rtype(result = "WormholeResult<usize>")]
struct SendAsk(Ask);

impl Handler<SendAsk> for ServerConnection {
    type Result = WormholeResult<usize>;

    fn handle(&mut self, SendAsk(ask): SendAsk, _ctx: &mut Context<Self>) -> Self::Result {
        use State::*;

        match &mut self.state {
            Connected(s, _) => {
                let ask_id = self.asks_sent;
                let msg = AskMessage { ask, ask_id };
                trace!("sending ask message: {:#?}", msg);

                s.write(Message::Text(serde_json::to_string(&msg)?))?;

                trace!("ask sent");
                self.asks_sent += 1;

                Ok(ask_id)
            }
            ConnectionLost => Err(WormholeError::ConnectionLost),
            NotConnected | WebsocketsConnected => Err(WormholeError::NeverConnected),
        }
    }
}

#[derive(actix::Message)]
#[rtype(result = "WormholeResult<()>")]
struct Connect(UserId);

impl Handler<Connect> for ServerConnection {
    type Result = ResponseFuture<WormholeResult<()>>;

    fn handle(&mut self, Connect(user_id): Connect, ctx: &mut Context<Self>) -> Self::Result {
        use crate::client::{client, SERVER_URL};
        use actix::{ActorFuture, WrapFuture};
        use State::*;

        match &self.state {
            Connected(_, _) | WebsocketsConnected => {
                warn!(
                    "ignoring connection request as current state is {:#?}",
                    self.state
                );
                return Box::pin(async move { Ok(()) });
            }
            s => debug!("connecting to wormhole, current state: {:#?}", s),
        };

        let (tx, rx) = oneshot::channel::<WormholeResult<()>>();
        self.user = Some(user_id.clone());
        let req = &EstablishWormholeRequest { user_id };

        ctx.spawn(
            client()
                .ws(format!("{}/{}", *SERVER_URL, "wormhole"))
                .header(
                    "EstablishWormholeRequest",
                    match serde_json::to_string(req) {
                        Err(e) => return Box::pin(async move { Err(e.into()) }),
                        Ok(j) => j,
                    },
                )
                .connect()
                .into_actor(self)
                .then(|res, act, ctx| {
                    match res {
                        Err(e) => tx.send(Err(e.into())).unwrap(),
                        Ok((_, framed)) => {
                            let (sink, stream) = framed.split();
                            act.state = State::Connected(SinkWrite::new(sink, ctx), Instant::now());
                            ServerConnection::add_stream(stream, ctx);

                            // start heartbeats, otherwise server will disconnect after 10 seconds
                            act.heartbeat(ctx);

                            tx.send(Ok(())).unwrap();
                        }
                    };
                    actix::fut::ready(())
                }),
        );

        Box::pin(async move {
            match rx.await {
                Err(e) => Err(e.into()),
                Ok(_) => Ok(()),
            }
        })
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for ServerConnection {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        self.notes.push_back(match msg {
            Ok(Frame::Text(s)) => {
                let note: WormholeResult<Note> = std::str::from_utf8(&s)
                    .map_err(|e| WormholeError::Utf8(e))
                    .and_then(|s| serde_json::from_str(&s).map_err(|e| e.into()));

                if let Ok(ref note) = note {
                    // Handlers that want to consume this note can simply consume it,
                    // or take it, in which case we need the index of the handler to remove it.
                    #[derive(Debug)]
                    enum GreedyAction {
                        Consume,
                        Take(usize),
                    }

                    // go through our note handlers and see if any want to greedily take this note
                    let greedy_action =
                        self.note_handlers
                            .iter_mut()
                            .enumerate()
                            .find_map(|(i, uh)| {
                                Some(match uh.continue_handling(note) {
                                    Some(ContinueBehavior::Pass) => return None,
                                    Some(ContinueBehavior::Consume) => GreedyAction::Consume,
                                    None => GreedyAction::Take(i),
                                })
                            });

                    if let Some(action) = greedy_action {
                        trace!("consuming note {:#?}; greedy action: {:#?}", note, action);
                        match action {
                            GreedyAction::Consume => return,
                            GreedyAction::Take(i) => {
                                self.note_handlers.remove(i);
                                return;
                            }
                        }
                    }
                }

                trace!("queueing note: {:#?}", note);
                note
            }
            Ok(Frame::Pong(_)) => {
                self.state.update_heartbeat();
                return;
            }
            Ok(_) => return,
            Err(e) => Err(e.into()),
        });
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("Wormhole connection established!");

        self.state = match std::mem::take(&mut self.state) {
            State::Connected(s, hb) => State::Connected(s, hb),
            _ => State::WebsocketsConnected,
        };
    }

    fn finished(&mut self, _ctx: &mut Context<Self>) {
        error!("Server disconnected (user {:#?})", self.user);
        self.state = State::ConnectionLost;
    }
}

impl actix::io::WriteHandler<WsProtocolError> for ServerConnection {}
