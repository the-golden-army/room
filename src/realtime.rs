use actix::prelude::*;
use actix_web::{web, Error, HttpRequest};
use actix_web_actors::ws;
use futures_util::StreamExt;
use serde_json::{json, Value};

use std::time::Instant;

use crate::state::AppState;

pub struct Connect {
    pub user_id: i64,
    pub addr: Addr<Session>,
}
impl Message for Connect {
    type Result = ();
}

pub struct Disconnect {
    pub addr: Addr<Session>,
}
impl Message for Disconnect {
    type Result = ();
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Push(pub Value);

#[derive(Message)]
#[rtype(result = "()")]
pub struct ToUser {
    pub user_id: i64,
    pub value: Value,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ToUsers {
    pub users: Vec<i64>,
    pub value: Value,
}

pub struct Hub {
    sessions: Vec<(Addr<Session>, i64)>,
}

impl Hub {
    pub fn new() -> Self {
        Hub { sessions: Vec::new() }
    }

    pub fn online_ids(&self) -> Vec<i64> {
        let mut v: Vec<i64> = self.sessions.iter().map(|(_, u)| *u).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

impl Actor for Hub {
    type Context = Context<Self>;
}

impl Handler<Connect> for Hub {
    type Result = ();
    fn handle(&mut self, msg: Connect, _ctx: &mut Self::Context) {
        self.sessions.push((msg.addr, msg.user_id));
    }
}

impl Handler<Disconnect> for Hub {
    type Result = ();
    fn handle(&mut self, msg: Disconnect, _ctx: &mut Self::Context) {
        self.sessions.retain(|(a, _)| *a != msg.addr);
    }
}

impl Handler<Push> for Hub {
    type Result = ();
    fn handle(&mut self, msg: Push, _ctx: &mut Self::Context) {
        for (addr, _) in self.sessions.iter() {
            addr.do_send(WsPush(msg.0.clone()));
        }
    }
}

impl Handler<ToUser> for Hub {
    type Result = ();
    fn handle(&mut self, msg: ToUser, _ctx: &mut Self::Context) {
        for (addr, uid) in self.sessions.iter() {
            if *uid == msg.user_id {
                addr.do_send(WsPush(msg.value.clone()));
            }
        }
    }
}

impl Handler<ToUsers> for Hub {
    type Result = ();
    fn handle(&mut self, msg: ToUsers, _ctx: &mut Self::Context) {
        for (addr, uid) in self.sessions.iter() {
            if msg.users.contains(uid) {
                addr.do_send(WsPush(msg.value.clone()));
            }
        }
    }
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct WsPush(pub Value);

pub struct Session {
    pub user_id: i64,
    pub hub: Addr<Hub>,
    pub state: web::Data<AppState>,
    pub hb: Instant,
    pub authed: bool,
}

impl Session {
    pub fn new(user_id: i64, hub: Addr<Hub>, state: web::Data<AppState>) -> Self {
        Session {
            user_id,
            hub,
            state,
            hb: Instant::now(),
            authed: false,
        }
    }

    fn send_json(&self, ctx: &mut ws::WebsocketContext<Self>, v: Value) {
        if let Ok(s) = serde_json::to_string(&v) {
            ctx.text(s);
        }
    }
}

impl Actor for Session {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hub.do_send(Connect {
            user_id: self.user_id,
            addr: ctx.address(),
        });
        let n = self.state.clone();
        let uid = self.user_id;
        actix::spawn(async move {
            if let Ok(u) = crate::store::get_user(&n.pool, uid).await {
                let _ = crate::store::set_presence(&n.pool, uid, "online").await;
                let _ = crate::store::touch_user(&n.pool, uid).await;
                let _ = u;
            }
            let snap = crate::store::presence_snapshot(&n.pool).await;
            n.broadcast(json!({"t": "presence", "users": snap.online}));
        });
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.hub.do_send(Disconnect {
            addr: ctx.address(),
        });
    }
}

impl Handler<WsPush> for Session {
    type Result = ();
    fn handle(&mut self, msg: WsPush, ctx: &mut Self::Context) {
        if let Ok(s) = serde_json::to_string(&msg.0) {
            ctx.text(s);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Session {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(m)) => {
                self.hb = Instant::now();
                ctx.pong(&m);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                self.hb = Instant::now();
                let v: Value = match serde_json::from_str(text.trim()) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                let state = self.state.clone();
                let uid = self.user_id;
                let hub = self.hub.clone();
                let addr = ctx.address();
                actix::spawn(async move {
                    match crate::api::ws_event(&state, &hub, uid, v).await {
                        Ok(Some(out)) => {
                            if let Ok(s) = serde_json::to_string(&out) {
                                addr.do_send(RawText(s));
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            let msg = json!({"t": "error", "message": e});
                            if let Ok(s) = serde_json::to_string(&msg) {
                                addr.do_send(RawText(s));
                            }
                        }
                    }
                });
            }
            Ok(ws::Message::Binary(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            _ => (),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RawText(pub String);

impl Handler<RawText> for Session {
    type Result = ();
    fn handle(&mut self, msg: RawText, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

pub async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<actix_web::HttpResponse, Error> {
    let user_id = crate::auth::resolve(&req, &state).await.map(|a| a.user.id).unwrap_or(0);
    let hub = state.hub.clone();
    let sess = Session::new(user_id, hub, state);
    ws::start(sess, &req, stream)
}
