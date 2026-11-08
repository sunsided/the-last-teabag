//! Where the story's rooms and things are on screen. The story knows none of this.

use crate::art::*;

pub struct Room {
    pub id: &'static str,
    pub x: (f32, f32),
    pub floor: f32,
}

pub const ROOMS: &[Room] = &[
    Room { id: "attic", x: (130.0, 300.0), floor: 62.0 },
    Room { id: "bedroom", x: (66.0, 196.0), floor: 102.0 },
    Room { id: "bathroom", x: (206.0, 334.0), floor: 102.0 },
    Room { id: "study", x: (66.0, 196.0), floor: 142.0 },
    Room { id: "kitchen", x: (206.0, 334.0), floor: 142.0 },
    Room { id: "cellar", x: (160.0, 334.0), floor: 180.0 },
];

/// A way between two rooms: leave `a` at `ax` by `dir`, arrive in `b` at `bx`.
pub struct Portal {
    pub a: &'static str,
    pub ax: f32,
    pub dir: &'static str,
    pub back: &'static str,
    pub b: &'static str,
    pub bx: f32,
}

impl Portal {
    /// Where to stand at `a` and at `b`. Stairs descend left to right as drawn,
    /// so the top end is left of the portal's x and the bottom end right of it.
    fn ends(&self) -> (f32, f32) {
        let (a, b) = (room(self.a).map_or(0.0, |r| r.floor), room(self.b).map_or(0.0, |r| r.floor));
        if a == b {
            (self.ax, self.bx)
        } else if a < b {
            (self.ax - STAIR_RUN, self.bx + STAIR_RUN)
        } else {
            (self.ax + STAIR_RUN, self.bx - STAIR_RUN)
        }
    }
}

const STAIR_RUN: f32 = 8.0;

pub const PORTALS: &[Portal] = &[
    Portal { a: "bedroom", ax: 150.0, dir: "up", back: "down", b: "attic", bx: 150.0 },
    Portal { a: "bedroom", ax: 196.0, dir: "east", back: "west", b: "bathroom", bx: 206.0 },
    Portal { a: "bathroom", ax: 326.0, dir: "down", back: "up", b: "kitchen", bx: 326.0 },
    Portal { a: "study", ax: 196.0, dir: "east", back: "west", b: "kitchen", bx: 206.0 },
    Portal { a: "kitchen", ax: 230.0, dir: "down", back: "up", b: "cellar", bx: 230.0 },
];

/// One end of a portal, as something to click.
pub struct Exit {
    pub portal: usize,
    pub room: &'static str,
    /// Where to stand to use it.
    pub x: f32,
    /// The middle of what is drawn, for clicking.
    spot: f32,
    pub dir: &'static str,
    pub to: &'static str,
    pub arrive_x: f32,
}

impl Exit {
    pub fn id(&self) -> String {
        format!("exit:{}:{}", self.portal, self.room)
    }

    pub fn name(&self) -> String {
        match self.dir {
            "up" | "down" => format!("stairs {}", self.dir),
            _ => format!("door to {}", self.to),
        }
    }

    fn hit(&self, x: f32, y: f32) -> bool {
        let (Some(here), Some(there)) = (room(self.room), room(self.to)) else {
            return false;
        };
        // The top end of a stair is a hatch in the floor; the rest is full height.
        let top = if here.floor < there.floor { here.floor - 14.0 } else { here.floor - 35.0 };
        (x - self.spot).abs() <= 8.0 && y >= top && y <= here.floor + 3.0
    }
}

pub fn exits() -> Vec<Exit> {
    PORTALS
        .iter()
        .enumerate()
        .flat_map(|(portal, p)| {
            let (a, b) = p.ends();
            [
                Exit { portal, room: p.a, x: a, spot: p.ax, dir: p.dir, to: p.b, arrive_x: b },
                Exit { portal, room: p.b, x: b, spot: p.bx, dir: p.back, to: p.a, arrive_x: a },
            ]
        })
        .collect()
}

pub fn exit_at(x: f32, y: f32) -> Option<Exit> {
    exits().into_iter().find(|e| e.hit(x, y))
}

pub fn room(id: &str) -> Option<&'static Room> {
    ROOMS.iter().find(|r| r.id == id)
}

/// The room someone standing at `pos` is in; `None` on stairs and in doorways.
pub fn standing_in(pos: [f32; 2]) -> Option<&'static Room> {
    ROOMS.iter().find(|r| (pos[1] - r.floor).abs() < 0.5 && pos[0] >= r.x.0 - 2.0 && pos[0] <= r.x.1 + 2.0)
}

pub fn room_at(x: f32, y: f32) -> Option<&'static Room> {
    ROOMS.iter().find(|r| x >= r.x.0 - 2.0 && x <= r.x.1 + 2.0 && y <= r.floor && y > r.floor - 38.0)
}

/// One leg of a route: the x to walk to in the current room, the `go`
/// direction there, the room it leads to, and the x of arrival.
pub struct Leg {
    pub leave_x: f32,
    pub dir: &'static str,
    pub to: &'static str,
    pub arrive_x: f32,
}

/// Shortest route by rooms (breadth-first; the house is six rooms).
pub fn route(from: &str, to: &str) -> Option<Vec<Leg>> {
    let mut seen = vec![from.to_string()];
    let mut queue = std::collections::VecDeque::from([(from.to_string(), Vec::new())]);
    while let Some((at, legs)) = queue.pop_front() {
        if at == to {
            return Some(legs);
        }
        for p in PORTALS {
            let leg = if p.a == at {
                Leg { leave_x: p.ends().0, dir: p.dir, to: p.b, arrive_x: p.ends().1 }
            } else if p.b == at {
                Leg { leave_x: p.ends().1, dir: p.back, to: p.a, arrive_x: p.ends().0 }
            } else {
                continue;
            };
            if seen.iter().any(|s| s == leg.to) {
                continue;
            }
            seen.push(leg.to.to_string());
            let mut next: Vec<Leg> = legs.iter().map(Leg::copy).collect();
            next.push(leg);
            queue.push_back((seen.last().unwrap().clone(), next));
        }
    }
    None
}

impl Leg {
    fn copy(&self) -> Leg {
        Leg { leave_x: self.leave_x, dir: self.dir, to: self.to, arrive_x: self.arrive_x }
    }
}

/// A clickable thing. `sprite` is drawn with its feet on the floor at `x`.
pub struct Spot {
    pub id: &'static str,
    pub places: &'static [(&'static str, f32)],
    pub sprite: &'static [&'static str],
    pub default_verb: &'static str,
}

pub const SPOTS: &[Spot] = &[
    Spot { id: "bed", places: &[("bedroom", 92.0)], sprite: BED, default_verb: "examine" },
    Spot { id: "specs", places: &[("bedroom", 124.0)], sprite: SPECS, default_verb: "take" },
    Spot { id: "tap", places: &[("bathroom", 250.0)], sprite: TAP, default_verb: "use" },
    Spot { id: "desk", places: &[("study", 110.0)], sprite: DESK, default_verb: "examine" },
    Spot { id: "stove", places: &[("kitchen", 290.0)], sprite: STOVE, default_verb: "use" },
    Spot { id: "sink", places: &[("kitchen", 262.0)], sprite: SINK, default_verb: "use" },
    Spot { id: "kettle", places: &[("kitchen", 290.0)], sprite: KETTLE, default_verb: "take" },
    Spot { id: "mug", places: &[("kitchen", 262.0), ("study", 110.0)], sprite: MUG, default_verb: "take" },
    Spot { id: "piano", places: &[("attic", 240.0)], sprite: PIANO, default_verb: "use" },
    Spot { id: "chest", places: &[("cellar", 280.0)], sprite: CHEST, default_verb: "open" },
    Spot { id: "poster", places: &[("attic", 255.0)], sprite: POSTER, default_verb: "examine" },
    Spot { id: "darkness", places: &[("cellar", 167.0)], sprite: DARKNESS, default_verb: "examine" },
    Spot { id: "teabag", places: &[("cellar", 280.0)], sprite: TEABAG, default_verb: "take" },
];

/// Where to stand to reach a thing at `x`: beside it and its furniture, not in front.
pub fn stand_x(room_id: &str, x: f32) -> f32 {
    let width = SPOTS
        .iter()
        .filter(|s| s.places.iter().any(|p| p.0 == room_id && (p.1 - x).abs() < 1.0))
        .flat_map(|s| s.sprite.iter().map(|r| r.len()))
        .max()
        .unwrap_or(0) as f32;
    let left = x - width / 2.0 - 6.0;
    // In a left corner there is no room on that side, so stand on the other.
    if room(room_id).is_some_and(|r| left < r.x.0) { x + width / 2.0 + 6.0 } else { left }
}

/// Things that stand on furniture draw raised by this much.
pub fn lift(id: &str) -> f32 {
    match id {
        "kettle" => 12.0,
        "mug" | "specs" => 11.0,
        "teabag" | "poster" => 12.0,
        _ => 0.0,
    }
}
