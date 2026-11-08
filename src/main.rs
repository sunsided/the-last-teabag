//! A point-and-click host for a loremaster story: the story decides what
//! happens, this file decides where people stand while it does.

mod house;

use game_toolkit::prelude::*;
use house::{PLAYER as HERO, OTTO as OTTO_ART, SPOTS};
use loremaster_model::{Id, Location, Value};
use loremaster_runtime::{Block, BlockKind, Command, Effect, Input, Session, Status, Step, World};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

const W: f32 = 400.0;
const H: f32 = 250.0;
const SPEED: f32 = 60.0;
const TICK_SECS: f32 = 6.0;
const BAR_Y: f32 = 192.0;

const VERBS: [(&str, &str); 7] = [
    ("walk", "Walk to"),
    ("examine", "Look at"),
    ("take", "Pick up"),
    ("open", "Open"),
    ("use", "Use"),
    ("give", "Give"),
    ("talk", "Talk to"),
];

enum Do {
    Walk(f32, f32),
    Go(String),
    Perform(Id, Vec<Id>),
    /// Walk to a person who may have moved: plan again from here.
    Chase(Id, Vec<Id>, Id),
}

#[derive(Default)]
struct Actor {
    pos: [f32; 2],
    path: VecDeque<[f32; 2]>,
    flip: bool,
    phase: f32,
}

impl Actor {
    fn advance(&mut self, dt: f32) -> bool {
        let Some(&t) = self.path.front() else { return true };
        let (dx, dy) = (t[0] - self.pos[0], t[1] - self.pos[1]);
        let d = (dx * dx + dy * dy).sqrt();
        let step = SPEED * dt;
        if dx.abs() > 0.5 {
            self.flip = dx < 0.0;
        }
        self.phase += dt * 8.0;
        if d <= step {
            self.pos = t;
            self.path.pop_front();
            return self.path.is_empty();
        }
        self.pos = [self.pos[0] + dx / d * step, self.pos[1] + dy / d * step];
        false
    }
}

struct Line {
    text: String,
    who: Option<String>,
    left: f32,
    shown: f32,
}

struct App {
    session: Session,
    names: HashMap<String, String>,
    art: HashMap<&'static str, (TextureId, u32, u32)>,
    hero: Actor,
    otto: Actor,
    otto_room: String,
    plan: VecDeque<Do>,
    /// The verb the player picked; `None` means each thing's suggested action.
    verb: Option<usize>,
    first: Option<Id>,
    lines: VecDeque<Line>,
    choices: Vec<String>,
    ending: Option<String>,
    tick: f32,
    /// After a skipped line, clicks wait a moment so a rage-click cannot act again.
    cooldown: f32,
}

fn texture(ctx: &mut Context, rows: &[&str]) -> (TextureId, u32, u32) {
    let w = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let mut rgba = Vec::with_capacity(w * rows.len() * 4);
    for r in rows {
        for x in 0..w {
            match r.as_bytes().get(x).and_then(|&c| house::color(c as char)) {
                Some(c) => rgba.extend([(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]),
                None => rgba.extend([0, 0, 0, 0]),
            }
        }
    }
    (ctx.gfx.create_texture_rgba(w as u32, rows.len() as u32, &rgba, None), w as u32, rows.len() as u32)
}

fn load_story() -> Result<(Arc<World>, HashMap<String, String>)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("story");
    let project = loremaster_project::load(loremaster_project::read_dir(&dir)?)
        .map_err(|e| anyhow::anyhow!("story does not load: {e:#?}"))?;
    let names = project.story.things.iter().map(|t| (t.id.to_string(), t.name.clone())).collect();
    Ok((Arc::new(World::compile(project.story)?), names))
}

fn loc(s: &Session, thing: &str) -> Option<Location> {
    s.snapshot().state().location_of(thing).cloned()
}

fn flag(s: &Session, name: &str) -> bool {
    matches!(s.snapshot().state().facts.get(name), Some(Value::Bool(true)))
}

impl App {
    fn hero_room(&self) -> String {
        self.session.snapshot().state().room.to_string()
    }

    /// Somebody on screen is still saying something.
    fn speaking(&self) -> bool {
        self.lines.iter().any(|l| l.who.is_some())
    }

    fn talking(&self) -> bool {
        !self.choices.is_empty()
    }

    fn say(&mut self, text: String, who: Option<String>) {
        let left = 1.5 + text.len() as f32 * 0.05;
        self.lines.push_back(Line { text, who, left, shown: 0.0 });
    }

    /// Takes in what the story said, and moves Otto's sprite to where the
    /// story now has him.
    fn take(&mut self, step: Step) {
        self.choices.clear();
        for e in step.effects {
            match e {
                Effect::Text { block } => {
                    let from = |prefix: &str| {
                        block.runs.iter().any(|r| r.source.as_ref().is_some_and(|s| s.owner.contains(prefix)))
                    };
                    let spoken = block.runs.iter().find_map(|r| r.speaker.clone());
                    let who = match block.kind {
                        BlockKind::Said => Some("player".to_string()),
                        BlockKind::Speech => Some("otto".to_string()),
                        BlockKind::End => None,
                        // Room text: the screen already shows the room.
                        BlockKind::Prose if from("room:") => continue,
                        BlockKind::Prose | BlockKind::Response | BlockKind::Narration => {
                            // What happens elsewhere has no mouth; the rest the player remarks on.
                            spoken.or_else(|| (!is_ambient(&block)).then(|| "player".to_string()))
                        }
                        _ => continue,
                    };
                    let text = block.plain_text();
                    // "comes in" notes describe what the sprite is already doing.
                    if !text.trim().is_empty() && !text.starts_with("Uncle Otto") {
                        self.say(text, who);
                    }
                }
                Effect::Choices { choices, .. } => {
                    self.choices = choices.into_iter().map(|c| c.text).collect();
                }
                Effect::Ending { ending } => self.ending = Some(ending.title),
                _ => {}
            }
        }
        if let Status::Ended { ending: Some(e) } = step.status {
            self.ending = Some(e.title);
        }
        self.sync_otto();
    }

    fn sync_otto(&mut self) {
        let Some(Location::Room(now)) = loc(&self.session, "otto") else { return };
        if now.as_str() == self.otto_room {
            return;
        }
        let Some(legs) = house::route(&self.otto_room, now.as_str()) else { return };
        let mut floor = house::room(&self.otto_room).map_or(0.0, |r| r.floor);
        for leg in legs {
            self.otto.path.push_back([leg.leave_x, floor]);
            floor = house::room(leg.to).map_or(floor, |r| r.floor);
            self.otto.path.push_back([leg.arrive_x, floor]);
        }
        let r = house::room(now.as_str()).expect("otto stays in the house");
        self.otto.path.push_back([(r.x.0 + r.x.1) / 2.0, r.floor]);
        self.otto_room = now.to_string();
    }

    /// Queue the walk to `x` in `room`, then `then`.
    fn plan_to(&mut self, room: &str, x: f32, then: Option<Do>) {
        self.plan.clear();
        self.hero.path.clear();
        let here = self.hero_room();
        let Some(legs) = house::route(&here, room) else { return };
        let mut floor = house::room(&here).map_or(0.0, |r| r.floor);
        for leg in legs {
            self.plan.push_back(Do::Walk(leg.leave_x, floor));
            self.plan.push_back(Do::Go(leg.dir.to_string()));
            floor = house::room(leg.to).map_or(floor, |r| r.floor);
            self.plan.push_back(Do::Walk(leg.arrive_x, floor));
        }
        self.plan.push_back(Do::Walk(x, floor));
        self.plan.extend(then);
    }

    fn perform(&mut self, verb: Id, objects: Vec<Id>) {
        let step = self.session.submit(Input::Command(Command::verb(verb, objects)));
        self.take(step);
    }

    /// Runs the plan until something has to wait for the walk.
    fn run_plan(&mut self) {
        while self.hero.path.is_empty() {
            match self.plan.pop_front() {
                None => return,
                Some(Do::Walk(x, y)) => self.hero.path.push_back([x, y]),
                Some(Do::Go(dir)) => {
                    let step = self.session.submit(Input::Command(Command::Go { direction: dir }));
                    self.take(step);
                }
                Some(Do::Perform(v, o)) => self.perform(v, o),
                Some(Do::Chase(v, o, who)) => {
                    let Some(Location::Room(there)) = loc(&self.session, who.as_str()) else { return };
                    if there.as_str() == self.hero_room() {
                        self.hero.path.push_back([self.otto.pos[0] - 14.0, self.hero.pos[1]]);
                        self.plan.push_front(Do::Perform(v, o));
                    } else {
                        let x = self.otto.pos[0];
                        self.plan_to(there.as_str(), x, Some(Do::Chase(v, o, who.clone())));
                    }
                }
            }
        }
    }

    fn at(&self, x: f32, y: f32) -> Option<(Id, String, f32)> {
        let o = self.otto.pos;
        if (x - o[0]).abs() <= 6.0 && y <= o[1] && y >= o[1] - 18.0 {
            return Some(("otto".into(), self.otto_room.clone(), o[0]));
        }
        for s in SPOTS {
            let Some((room, ax)) = self.place(s) else { continue };
            let (_, w, h) = self.art[s.id];
            let r = house::room(&room)?;
            let top = r.floor - house::lift(s.id) - h as f32;
            if x >= ax - w as f32 / 2.0 && x <= ax + w as f32 / 2.0 && y >= top && y <= top + h as f32 {
                let stand = house::stand_x(&room, ax);
                return Some((s.id.into(), room, stand));
            }
        }
        house::exit_at(x, y).map(|e| (e.id().as_str().into(), e.room.to_string(), e.x))
    }

    fn place(&self, s: &house::Spot) -> Option<(String, f32)> {
        let room = match loc(&self.session, s.id)? {
            Location::Room(r) => r.to_string(),
            Location::In(c) if flag(&self.session, &format!("{c}.open")) => s.places[0].0.to_string(),
            _ => return None,
        };
        let x = s.places.iter().find(|p| p.0 == room).map(|p| p.1)?;
        Some((room, x))
    }

    fn inventory(&self) -> Vec<Id> {
        let snap = self.session.snapshot();
        snap.state().locations.iter().filter(|(_, l)| **l == Location::Carried).map(|(i, _)| i.clone()).collect()
    }

    fn name(&self, id: &str) -> String {
        if let Some(e) = house::exits().into_iter().find(|e| e.id() == id) {
            return e.name();
        }
        // The scene has the name the story gives now; other rooms keep the base name.
        let scene = self.session.scene();
        scene.things.iter().chain(&scene.inventory).find(|t| t.id == id).map(|t| t.name.clone())
            .or_else(|| self.names.get(id).cloned())
            .unwrap_or_else(|| id.to_string())
    }

    /// The action a click would run without a picked verb.
    fn suggested(&self, id: &str, carried: bool) -> &'static str {
        if carried {
            "examine"
        } else if id == "otto" {
            "talk"
        } else if id.starts_with("exit:") {
            "use"
        } else {
            SPOTS.iter().find(|s| s.id == id).map_or("examine", |s| s.default_verb)
        }
    }

    /// The verb shown as active while the mouse is over `hover`.
    fn active(&self, hover: Option<&(Id, bool)>) -> usize {
        self.verb.unwrap_or_else(|| {
            hover
                .and_then(|(id, carried)| {
                    let verb = self.suggested(id.as_str(), *carried);
                    VERBS.iter().position(|v| v.0 == verb)
                })
                .unwrap_or(0)
        })
    }

    fn sentence(&self, hover: Option<&(Id, bool)>) -> String {
        let (key, verb) = VERBS[self.active(hover)];
        let hover = hover.map(|h| h.0.as_str());
        let join = if key == "give" { "to" } else { "with" };
        match (&self.first, hover) {
            (Some(f), Some(h)) => format!("{verb} {} {join} {}", self.name(f.as_str()), self.name(h)),
            (Some(f), None) => format!("{verb} {} {join}", self.name(f.as_str())),
            (None, Some(h)) => format!("{verb} {}", self.name(h)),
            (None, None) => verb.to_string(),
        }
    }

    /// A thing was chosen with a verb (left click) or as a default (right click).
    fn act(&mut self, id: Id, room: Option<(String, f32)>, verb: &str, objects: Vec<Id>) {
        let verb: Id = verb.into();
        let person = id.as_str() == "otto";
        match room {
            None => self.perform(verb, objects),
            Some(_) if person => {
                self.plan.clear();
                self.hero.path.clear();
                self.plan.push_back(Do::Chase(verb, objects, id));
            }
            Some((room, x)) => self.plan_to(&room, x, Some(Do::Perform(verb, objects))),
        }
        self.first = None;
        self.verb = None;
    }

    fn click_exit(&mut self, exit: house::Exit, verb: &str) {
        match verb {
            "use" | "open" => {
                self.plan_to(exit.room, exit.x, Some(Do::Go(exit.dir.to_string())));
                let floor = house::room(exit.to).map_or(0.0, |r| r.floor);
                self.plan.push_back(Do::Walk(exit.arrive_x, floor));
            }
            "walk" => self.plan_to(exit.room, exit.x, None),
            "examine" => self.say(format!("It leads to the {}.", exit.to), Some("player".to_string())),
            _ => self.say("That won't work.".to_string(), Some("player".to_string())),
        }
        self.first = None;
        self.verb = None;
    }

    fn click_thing(&mut self, id: Id, room: Option<(String, f32)>) {
        let verb = self.verb.map_or_else(|| self.suggested(id.as_str(), room.is_none()), |i| VERBS[i].0);
        if let Some(exit) = house::exits().into_iter().find(|e| e.id() == id.as_str()) {
            return self.click_exit(exit, verb);
        }
        match (verb, self.first.clone()) {
            ("walk", _) => {
                if let Some((r, x)) = room {
                    self.plan_to(&r, x, None);
                }
                self.verb = None;
            }
            ("use" | "give", None) if room.is_none() && self.verb.is_some() => self.first = Some(id),
            ("use" | "give", Some(f)) if f == id => self.act(id, room, verb, vec![f]),
            ("use" | "give", Some(f)) => self.act(id.clone(), room, verb, vec![f, id]),
            (_, _) => self.act(id.clone(), room, verb, vec![id]),
        }
    }

    fn click(&mut self, ctx: &Context, left: bool) {
        let (x, y) = ctx.input.mouse_pos();
        if self.ending.is_some() {
            let step = self.session.restart();
            self.ending = None;
            self.lines.clear();
            self.reset_sprites();
            self.take(step);
            return;
        }
        if self.cooldown > 0.0 {
            return;
        }
        // Nobody acts or answers before the speaker has finished; a click only skips the line.
        if self.speaking() || (self.talking() && !self.lines.is_empty()) {
            if self.lines.front().is_some_and(|l| l.shown >= 0.4) {
                self.lines.pop_front();
                if !self.speaking() {
                    self.cooldown = 0.3;
                }
            }
            return;
        }
        if self.talking() {
            if left && y >= BAR_Y {
                let i = ((y - BAR_Y - 2.0) / 11.0) as usize;
                if i < self.choices.len() {
                    let step = self.session.submit(Input::Choice(i));
                    self.take(step);
                }
            }
            return;
        }
        if !left {
            self.verb = None;
            self.first = None;
        }
        if y >= BAR_Y + 10.0 {
            if x < 180.0 {
                let (c, r) = (((x - 4.0) / 58.0) as usize, ((y - BAR_Y - 12.0) / 14.0) as usize);
                if left && c < 3 && r * 3 + c < VERBS.len() {
                    self.verb = Some(r * 3 + c);
                    self.first = None;
                }
            } else if let Some(id) = self.inventory_at(x, y) {
                self.click_thing(id, None);
            }
            return;
        }
        match self.at(x, y) {
            Some((id, room, ax)) => self.click_thing(id, Some((room, ax))),
            None => {
                if self.first.is_none() && left {
                    if let Some(r) = house::room_at(x, y) {
                        self.plan_to(r.id, x.clamp(r.x.0, r.x.1), None);
                    }
                }
                self.first = None;
                self.verb = None;
            }
        }
    }

    fn inventory_at(&self, x: f32, y: f32) -> Option<Id> {
        let c = ((x - 184.0) / 108.0) as usize;
        let i = c + ((y - BAR_Y - 12.0) / 14.0) as usize * 2;
        (x >= 184.0 && c < 2).then(|| self.inventory().get(i).cloned()).flatten()
    }

    fn reset_sprites(&mut self) {
        let start = house::room("bedroom").expect("the house has a bedroom");
        self.hero = Actor { pos: [150.0, start.floor], ..Default::default() };
        self.otto = Actor { pos: [110.0, start.floor], ..Default::default() };
        self.otto_room = "bedroom".into();
        self.plan.clear();
    }
}

impl Game for App {
    fn init(ctx: &mut Context) -> Result<Self> {
        let (world, names) = load_story()?;
        let mut art = HashMap::new();
        for s in SPOTS {
            art.insert(s.id, texture(ctx, s.sprite));
        }
        art.insert("hero0", texture(ctx, HERO[0]));
        art.insert("hero1", texture(ctx, HERO[1]));
        art.insert("otto0", texture(ctx, OTTO_ART[0]));
        art.insert("otto1", texture(ctx, OTTO_ART[1]));
        let mut app = App {
            session: Session::new(world),
            names,
            art,
            hero: Actor::default(),
            otto: Actor::default(),
            otto_room: String::new(),
            plan: VecDeque::new(),
            verb: None,
            first: None,
            lines: VecDeque::new(),
            choices: Vec::new(),
            ending: None,
            tick: 0.0,
            cooldown: 0.0,
        };
        app.reset_sprites();
        let step = app.session.start();
        app.take(step);
        Ok(app)
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        if ctx.input.key_pressed(Key::Escape) {
            if self.talking() {
                let step = self.session.submit(Input::Command(Command::Leave));
                self.take(step);
            } else {
                ctx.quit();
            }
        }
        if ctx.input.key_pressed(Key::F12) {
            ctx.gfx.request_screenshot("screenshot.png");
        }
        for (b, left) in [(MouseButton::Left, true), (MouseButton::Right, false)] {
            if ctx.input.mouse_pressed(b) {
                self.click(ctx, left);
            }
        }
        if self.hero.advance(dt) {
            self.run_plan();
        }
        self.otto.advance(dt);
        self.cooldown = (self.cooldown - dt).max(0.0);
        if let Some(l) = self.lines.front_mut() {
            l.shown += dt;
            l.left -= dt;
            if l.left <= 0.0 {
                self.lines.pop_front();
            }
        }
        // Time passes only while nobody is mid-talk and Otto has caught up
        // with what the story already says he did.
        if self.ending.is_none() && !self.talking() && !self.speaking() && self.otto.path.is_empty() {
            self.tick += dt;
            if self.tick >= TICK_SECS {
                self.tick = 0.0;
                let step = self.session.submit(Input::Tick);
                self.take(step);
            }
        }
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let (mx, my) = ctx.input.mouse_pos();
        let hover: Option<(Id, bool)> = if my < BAR_Y {
            self.at(mx, my).map(|h| (h.0, false))
        } else if my >= BAR_Y + 10.0 {
            self.inventory_at(mx, my).map(|id| (id, true))
        } else {
            None
        };
        let active = self.active(hover.as_ref());
        let state = self.session.snapshot();
        let state = state.state();
        let minutes = state.minutes;
        let inv = self.inventory();
        let sentence = self.sentence(hover.as_ref());
        let hover = hover.map(|h| h.0);
        let mut p = frame.painter(&mut ctx.gfx);
        let tod = ((minutes % 1440) as f32 - 480.0) / 300.0;
        p.clear(Color::rgb(0.45 + tod.min(1.0) * 0.1, 0.65, 0.85));
        // Ground, with the cellar dug into it.
        p.rect([0.0, 144.0], [W, 48.0], Color::hex(0x4a3524));
        p.rect([0.0, 142.0], [W, 3.0], Color::hex(0x4f8a3f));
        // A tree, so the house stands somewhere.
        p.rect([24.0, 96.0], [8.0, 46.0], Color::hex(0x5a3a22));
        p.circle([28.0, 86.0], 22.0, Color::hex(0x3f7a3f));
        // Roof: a stepped silhouette over the attic, in tiles.
        for i in 0..18 {
            let (y, inset) = (22.0 + i as f32 * 1.0, 72.0 - i as f32 * 4.0);
            p.rect([60.0 + inset, y], [280.0 - inset * 2.0, 1.0], Color::hex(0x8a3a2a));
        }
        p.rect([60.0, 40.0], [280.0, 2.0], Color::hex(0x5a2a20));
        for r in house::ROOMS {
            let top = r.floor - 38.0;
            p.rect([r.x.0 - 4.0, top], [r.x.1 - r.x.0 + 8.0, 38.0], Color::hex(r.wall));
            p.rect([r.x.0 - 4.0, r.floor], [r.x.1 - r.x.0 + 8.0, 3.0], Color::hex(0x2a1e18));
        }
        // Stairs: every portal that changes floor.
        let lit = |i: usize| hover.as_ref().is_some_and(|h| h.as_str().starts_with(&format!("exit:{i}:")));
        for (i, g) in house::PORTALS.iter().enumerate() {
            let (Some(a), Some(b)) = (house::room(g.a), house::room(g.b)) else { continue };
            if a.floor != b.floor {
                let (top, bot) = (a.floor.min(b.floor), a.floor.max(b.floor));
                let color = if lit(i) { Color::hex(0xe8c84a) } else { Color::hex(0xa8703a) };
                for s in 0..8 {
                    let f = s as f32 / 8.0;
                    p.rect([g.ax - 8.0 + f * 16.0, top + f * (bot - top)], [4.0, 2.0], color);
                }
            } else {
                let color = if lit(i) { Color::hex(0xe8c84a) } else { Color::hex(0x5a3a22) };
                p.rect([g.ax + 1.0, a.floor - 24.0], [8.0, 24.0], color);
                p.rect([g.ax + 7.0, a.floor - 12.0], [1.0, 2.0], Color::hex(0xe8c84a));
            }
        }
        let draw = |p: &mut Painter, art: (TextureId, u32, u32), x: f32, y: f32, flip: bool, tint: Color| {
            let (t, w, h) = art;
            let mut s = SpriteInstance::at([(x - w as f32 / 2.0).round(), (y - h as f32).round()], [w as f32, h as f32]);
            if flip {
                s = s.flip_x();
            }
            s.color = tint.into();
            p.sprite_ex(t, s, Layer(1), BlendMode::Alpha);
        };
        for s in SPOTS {
            let Some((room, x)) = self.place(s) else { continue };
            let r = house::room(&room).expect("spots name real rooms");
            let tint = if hover.as_ref().is_some_and(|h| h.as_str() == s.id) { Color::rgb(1.0, 1.0, 0.6) } else { Color::WHITE };
            draw(&mut p, self.art[s.id], x, r.floor - house::lift(s.id), false, tint);
        }
        if flag(&self.session, "kettle.hot") && matches!(loc(&self.session, "kettle"), Some(Location::Room(_))) {
            p.rect([292.0, 114.0], [1.0, 4.0], Color::WHITE);
        }
        if flag(&self.session, "piano_playing") {
            let t = ctx.time.elapsed.as_secs_f32();
            p.pixel_text([250.0, 26.0 + (t * 3.0).sin() * 3.0], "~", 1, Color::WHITE);
        }
        let frame_of = |a: &Actor| if a.path.is_empty() { 0 } else { (a.phase as usize) % 2 };
        let otto_tint = if hover.as_ref().is_some_and(|h| h.as_str() == "otto") { Color::rgb(1.0, 1.0, 0.6) } else { Color::WHITE };
        draw(&mut p, self.art[["otto0", "otto1"][frame_of(&self.otto)]], self.otto.pos[0], self.otto.pos[1], self.otto.flip, otto_tint);
        draw(&mut p, self.art[["hero0", "hero1"][frame_of(&self.hero)]], self.hero.pos[0], self.hero.pos[1], self.hero.flip, Color::WHITE);
        if let Some(l) = self.lines.front() {
            let head = match l.who.as_deref() {
                Some("otto") => Some((self.otto.pos, Color::hex(0xe8c84a))),
                Some(_) => Some((self.hero.pos, Color::WHITE)),
                None => None,
            };
            if let Some((pos, color)) = head {
                let lines = wrap(l.text.trim_matches('"'), 30);
                let mut y = (pos[1] - 21.0 - lines.len() as f32 * 9.0).max(1.0);
                for chunk in lines {
                    let tw = chunk.len() as f32 * 6.0;
                    let x = (pos[0] - tw / 2.0).clamp(2.0, W - tw - 2.0).round();
                    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                        p.pixel_text_ex([x + dx, y + dy], &chunk, 1, Color::BLACK, 3);
                    }
                    p.pixel_text_ex([x, y], &chunk, 1, color, 4);
                    y += 9.0;
                }
            } else {
                let mut y = 2.0;
                for chunk in wrap(&l.text, 64) {
                    let tw = chunk.len() as f32 * 6.0;
                    p.rect([(W - tw) / 2.0 - 2.0, y - 1.0], [tw + 4.0, 10.0], Color::BLACK.with_alpha(0.7));
                    p.pixel_text_ex([(W - tw) / 2.0, y], &chunk, 1, Color::hex(0xc0e0ff), 3);
                    y += 10.0;
                }
            }
        }
        p.rect([0.0, BAR_Y], [W, H - BAR_Y], Color::hex(0x14101c));
        if self.talking() {
            if !self.lines.is_empty() {
                p.pixel_text([6.0, BAR_Y + 3.0], "...", 1, Color::hex(0x88c070));
            }
            for (i, c) in self.choices.iter().enumerate().filter(|_| self.lines.is_empty()) {
                let hot = my >= BAR_Y + 2.0 + i as f32 * 11.0 && my < BAR_Y + 13.0 + i as f32 * 11.0;
                let line: String = c.chars().take(64).collect();
                p.pixel_text([6.0, BAR_Y + 3.0 + i as f32 * 11.0], &line, 1, if hot { Color::hex(0xe8c84a) } else { Color::hex(0x88c070) });
            }
        } else {
            p.pixel_text([6.0, BAR_Y + 2.0], &sentence, 1, Color::hex(0x88c070));
            let hhmm = format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60);
            p.pixel_text([W - 34.0, BAR_Y + 2.0], &hhmm, 1, Color::hex(0xc0e0ff));
            for (i, (_, label)) in VERBS.iter().enumerate() {
                let (x, y) = (4.0 + (i % 3) as f32 * 58.0, BAR_Y + 14.0 + (i / 3) as f32 * 14.0);
                let hot = i == active;
                p.pixel_text([x, y], label, 1, if hot { Color::hex(0xe8c84a) } else { Color::hex(0xa0a0b0) });
            }
            for (i, id) in inv.iter().enumerate() {
                let (x, y) = (184.0 + (i % 2) as f32 * 108.0, BAR_Y + 14.0 + (i / 2) as f32 * 14.0);
                let n: String = self.name(id.as_str()).chars().take(17).collect();
                p.pixel_text([x, y], &n, 1, Color::hex(0xd8b070));
            }
        }
        if let Some(t) = &self.ending {
            p.rect([90.0, 90.0], [220.0, 50.0], Color::BLACK.with_alpha(0.85));
            p.pixel_text_ex([200.0 - t.len() as f32 * 6.0, 100.0], t, 2, Color::hex(0xe8c84a), 3);
            p.pixel_text_ex([140.0, 124.0], "click to start again", 1, Color::WHITE, 3);
        }
    }
}

/// Text nobody on screen says. Loremaster has no such tag yet (sunsided/loremaster#178):
/// agendas and the intro are ambient by origin, rules by the id prefix `ambient_`.
fn is_ambient(block: &Block) -> bool {
    block.runs.iter().any(|r| {
        r.source.as_ref().is_some_and(|s| {
            ["agenda:", "story:", "rule:ambient_"].iter().any(|p| s.owner.contains(p))
        })
    })
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    for word in text.split_whitespace() {
        let last = lines.last_mut().expect("starts with one line");
        if !last.is_empty() && last.len() + 1 + word.len() > width {
            lines.push(word.to_string());
        } else {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(word);
        }
    }
    lines
}

fn main() -> Result<()> {
    env_logger::init();
    run::<App>(AppConfig {
        title: "The Last Teabag".into(),
        width: 1200,
        height: 750,
        virtual_resolution: Some((W as u32, H as u32)),
        border_color: Color::rgb(0.05, 0.05, 0.07),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drawn house and the story must agree on every door and stair.
    #[test]
    fn every_portal_is_a_real_exit() {
        let (world, _) = load_story().unwrap();
        for p in house::PORTALS {
            for (from, dir, to) in [(p.a, p.dir, p.b), (p.b, p.back, p.a)] {
                let mut s = Session::new(world.clone());
                s.start();
                for leg in house::route("bedroom", from).unwrap() {
                    s.submit(Input::Command(Command::Go { direction: leg.dir.into() }));
                }
                s.submit(Input::Command(Command::Go { direction: dir.into() }));
                assert_eq!(s.snapshot().state().room.as_str(), to, "{from} {dir}");
            }
        }
    }

    #[test]
    fn leave_ends_a_talk() {
        let (world, _) = load_story().unwrap();
        let mut s = Session::new(world);
        s.start();
        s.submit(Input::Command(Command::verb("talk", vec!["otto".into()])));
        assert!(s.snapshot().state().talk.is_some());
        s.submit(Input::Command(Command::Leave));
        assert!(s.snapshot().state().talk.is_none());
    }

    #[test]
    fn nobody_stands_in_front_of_what_they_reach() {
        for s in SPOTS {
            let width = s.sprite.iter().map(|r| r.len()).max().unwrap() as f32;
            for (room, x) in s.places {
                assert!(x - house::stand_x(room, *x) >= width / 2.0 + 4.0, "{}", s.id);
            }
        }
    }

    #[test]
    fn ambient_text_is_told_from_player_remarks() {
        let (world, _) = load_story().unwrap();
        let mut s = Session::new(world);
        s.start();
        let blocks = |step: Step| -> Vec<Block> {
            step.effects.into_iter().filter_map(|e| match e {
                Effect::Text { block } => Some(block),
                _ => None,
            }).collect()
        };
        s.submit(Input::Command(Command::Go { direction: "up".into() }));
        let used = blocks(s.submit(Input::Command(Command::verb("use", vec!["piano".into()]))));
        assert!(used.iter().any(|b| b.plain_text().contains("You play") && !is_ambient(b)));
        let world: Vec<Block> = (0..60).flat_map(|_| blocks(s.submit(Input::Tick))).collect();
        for needle in ["Plates clatter", "church clock"] {
            assert!(world.iter().any(|b| b.plain_text().contains(needle) && is_ambient(b)), "{needle}");
        }
    }

    #[test]
    fn otto_remarks_on_the_piano_only_once() {
        let (world, _) = load_story().unwrap();
        let mut s = Session::new(world);
        s.start();
        s.submit(Input::Command(Command::Go { direction: "up".into() }));
        let mut play = || {
            let step = s.submit(Input::Command(Command::verb("use", vec!["piano".into()])));
            step.effects.iter().any(|e| matches!(e, Effect::Text { block }
                if block.runs.iter().any(|r| r.speaker.as_deref() == Some("otto"))))
        };
        assert!(play());
        assert!(!play());
    }

    #[test]
    fn route_crosses_the_house() {
        let dirs: Vec<_> = house::route("attic", "cellar").unwrap().iter().map(|l| l.dir).collect();
        assert_eq!(dirs, ["down", "east", "down", "down"]);
    }

    #[test]
    fn wrap_keeps_lines_short() {
        assert_eq!(wrap("aa bb cc", 5), ["aa bb", "cc"]);
    }
}
