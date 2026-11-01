//! An opt-in, paused side visit. Visitor bodies are separate from their
//! frozen board actors; ending the visit never rewrites a turn or position.
use super::*;
const BUY_RANGE: f32 = 4.5;
const OFFER_SECONDS: f32 = 5.;
const VISIT_SECONDS: f32 = 15.;
const RUN_SPEED: f32 = 8.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandyVanPhase {
    Offer,
    Visit,
}
#[derive(Clone, Debug, Serialize)]
pub struct CandyVanView {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub price: i32,
    pub range: f32,
    pub phase: CandyVanPhase,
    pub remaining: f32,
    pub joined_seats: Vec<u8>,
    pub bought_seats: Vec<u8>,
    pub can_join: bool,
    pub can_buy: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct CandyRunView {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub home_x: f32,
    pub home_z: f32,
    pub home_space: u8,
    pub remaining: f32,
    pub speed: f32,
}
struct CandyRun {
    view: CandyRunView,
    ground_space: u8,
    input: (f32, f32),
    input_age: f32,
}
pub(super) struct CandyVan {
    x: f32,
    z: f32,
    yaw: f32,
    bought: HashSet<u8>,
    joined: HashSet<u8>,
    runs: Vec<CandyRun>,
    phase: Option<CandyVanPhase>,
    remaining: f32,
    seed: u64,
}
impl CandyVan {
    pub(super) fn new(map: Map, seed: u64) -> Option<Self> {
        if map != Map::Frozen {
            return None;
        }
        let source: Value = serde_json::from_str(map.source()).expect("candy van location");
        let config = &source["candy_van"];
        let finite = |field: &str, default: f32| {
            config[field]
                .as_f64()
                .map(|n| n as f32)
                .filter(|n| n.is_finite())
                .unwrap_or(default)
        };
        Some(Self {
            x: finite("x", 6.),
            z: finite("z", -6.),
            yaw: finite("yaw", 0.),
            bought: HashSet::new(),
            joined: HashSet::new(),
            runs: vec![],
            phase: None,
            remaining: 0.,
            seed,
        })
    }
}
impl Board {
    pub fn candy_paused(&self) -> bool {
        self.candy_van
            .as_ref()
            .is_some_and(|van| van.phase.is_some())
    }
    pub fn candy_van_view(&self) -> Option<CandyVanView> {
        self.candy_van_view_for(Some(self.turn_seat()))
    }
    pub fn candy_van_view_for(&self, viewer: Option<u8>) -> Option<CandyVanView> {
        let van = self.candy_van.as_ref()?;
        let phase = van.phase?;
        let mut bought_seats: Vec<_> = van.bought.iter().copied().collect();
        bought_seats.sort_unstable();
        let mut joined_seats: Vec<_> = van.joined.iter().copied().collect();
        joined_seats.sort_unstable();
        let token = viewer.and_then(|seat| self.tokens.iter().find(|t| t.seat == seat));
        let run = viewer.and_then(|seat| van.runs.iter().find(|run| run.view.seat == seat));
        Some(CandyVanView {
            x: van.x,
            z: van.z,
            yaw: van.yaw,
            price: PRIZE_COST,
            range: BUY_RANGE,
            phase,
            remaining: van.remaining,
            joined_seats,
            bought_seats,
            can_join: phase == CandyVanPhase::Offer
                && token.is_some()
                && viewer.is_some_and(|seat| !van.joined.contains(&seat)),
            can_buy: phase == CandyVanPhase::Visit
                && token.is_some_and(|t| t.coins >= PRIZE_COST && !van.bought.contains(&t.seat))
                && run
                    .is_some_and(|run| (run.view.x - van.x).hypot(run.view.z - van.z) <= BUY_RANGE),
        })
    }
    pub fn candy_runs_view(&self) -> Vec<CandyRunView> {
        self.candy_van
            .as_ref()
            .map(|v| {
                v.runs
                    .iter()
                    .map(|r| {
                        let mut view = r.view.clone();
                        view.remaining = v.remaining;
                        view
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    pub(super) fn candy_visit_spot(&self, seat: u8) -> Option<(f32, f32)> {
        let run = self
            .candy_van
            .as_ref()?
            .runs
            .iter()
            .find(|r| r.view.seat == seat)?;
        Some((run.view.x, run.view.z))
    }
    pub(super) fn restock_candy_van(&mut self) {
        let Some(van) = self.candy_van.as_mut() else {
            return;
        };
        van.bought.clear();
        van.joined.clear();
        van.runs.clear();
        if self.manual_events {van.phase=None;van.remaining=0.;return;}
        let mut rng = StdRng::seed_from_u64(
            van.seed ^ u64::from(self.round).wrapping_mul(0x7a80_302d_d3a7_3521),
        );
        let visiting = self.round == 1 || rng.random_bool(0.38);
        van.phase = visiting.then_some(CandyVanPhase::Offer);
        van.remaining = if visiting { OFFER_SECONDS } else { 0. };
        let at = (van.x, van.z, van.yaw);
        if visiting {
            self.say(json!({"kind":"candy_van_arrived","x":at.0,"z":at.1,"yaw":at.2,"seconds":OFFER_SECONDS,"phase":"offer","round":self.round}));
        }
    }
    pub fn use_manual_events(&mut self){self.manual_events=true;self.restock_candy_van();}
    pub fn summon_candy_van(&mut self) -> Result<(), BoardError> {
        let Some(van) = self.candy_van.as_mut() else {
            return Err(BoardError::WrongStep);
        };
        if van.phase.is_some() {
            return Ok(());
        }
        van.phase = Some(CandyVanPhase::Offer);
        van.remaining = OFFER_SECONDS;
        van.joined.clear();
        van.runs.clear();
        let at = (van.x, van.z, van.yaw);
        self.say(json!({"kind":"candy_van_arrived","x":at.0,"z":at.1,"yaw":at.2,"seconds":OFFER_SECONDS,"phase":"offer","summoned":true}));
        Ok(())
    }
    pub fn join_candy_visit(&mut self, seat: u8) -> Result<(), BoardError> {
        if !self.tokens.iter().any(|t| t.seat == seat) {
            return Err(BoardError::UnknownTarget);
        }
        let van = self.candy_van.as_mut().ok_or(BoardError::WrongStep)?;
        if van.phase != Some(CandyVanPhase::Offer) {
            return Err(BoardError::WrongStep);
        }
        if van.joined.insert(seat) {
            self.say(json!({"kind":"candy_visit_joined","seat":seat}));
        }
        Ok(())
    }
    pub fn candy_visit_input(&mut self, seat: u8, x: f32, z: f32) -> Result<(), BoardError> {
        if !x.is_finite() || !z.is_finite() {
            return Err(BoardError::OutOfReach);
        }
        let van = self.candy_van.as_mut().ok_or(BoardError::WrongStep)?;
        if van.phase != Some(CandyVanPhase::Visit) {
            return Err(BoardError::WrongStep);
        }
        let run = van
            .runs
            .iter_mut()
            .find(|r| r.view.seat == seat)
            .ok_or(BoardError::WrongStep)?;
        let length = x.hypot(z).max(1.);
        run.input = (x / length, z / length);
        run.input_age = 0.;
        Ok(())
    }
    pub fn candy_van_tick(&mut self, dt: f32) -> bool {
        let seats: Vec<_> = self.tokens.iter().map(|t| t.seat).collect();
        self.candy_visit_tick(dt, &seats)
    }
    pub fn candy_visit_tick(&mut self, dt: f32, eligible: &[u8]) -> bool {
        if !dt.is_finite() || dt <= 0. {
            return false;
        }
        let Some(mut van) = self.candy_van.take() else {
            return false;
        };
        let Some(phase) = van.phase else {
            self.candy_van = Some(van);
            return false;
        };
        let dt = dt.min(0.05);
        let old = van.remaining;
        van.remaining = (van.remaining - dt).max(0.);
        van.joined.retain(|seat| eligible.contains(seat));
        let mut changed = old.ceil() != van.remaining.ceil();
        if phase == CandyVanPhase::Visit {
            van.runs.retain(|r| eligible.contains(&r.view.seat));
            for run in &mut van.runs {
                run.input_age += dt;
                if run.input_age > 0.35 {
                    continue;
                }
                let before = (run.view.x, run.view.z);
                let next = (
                    before.0 + run.input.0 * RUN_SPEED * dt,
                    before.1 + run.input.1 * RUN_SPEED * dt,
                );
                let at = if self.position_ground(next, run.ground_space) {
                    next
                } else if self.position_ground((next.0, before.1), run.ground_space) {
                    (next.0, before.1)
                } else if self.position_ground((before.0, next.1), run.ground_space) {
                    (before.0, next.1)
                } else {
                    before
                };
                run.view.x = at.0;
                run.view.z = at.1;
                changed |= at != before;
            }
        }
        if van.remaining <= 0. {
            changed = true;
            if phase == CandyVanPhase::Offer && !van.joined.is_empty() {
                let mut seats: Vec<_> = van.joined.iter().copied().collect();
                seats.sort_unstable();
                for seat in seats {
                    let home = self.actual_spot_of(seat);
                    let home_space = self.space_of(seat);
                    let ground_space = self
                        .layout
                        .iter()
                        .filter(|s| !s.under && !s.gated)
                        .min_by(|a, b| {
                            (a.x - home.0)
                                .hypot(a.z - home.1)
                                .total_cmp(&(b.x - home.0).hypot(b.z - home.1))
                        })
                        .map(|s| s.id)
                        .unwrap_or(home_space);
                    van.runs.push(CandyRun {
                        view: CandyRunView {
                            seat,
                            x: home.0,
                            z: home.1,
                            home_x: home.0,
                            home_z: home.1,
                            home_space,
                            remaining: VISIT_SECONDS,
                            speed: RUN_SPEED,
                        },
                        ground_space,
                        input: (0., 0.),
                        input_age: 1.,
                    });
                }
                van.phase = Some(CandyVanPhase::Visit);
                van.remaining = VISIT_SECONDS;
                self.say(json!({"kind":"candy_visit_started","seconds":VISIT_SECONDS,"joined_seats":van.runs.iter().map(|r|r.view.seat).collect::<Vec<_>>()}));
            } else {
                let joined: Vec<_> = van.runs.iter().map(|r| r.view.seat).collect();
                van.phase = None;
                van.runs.clear();
                van.joined.clear();
                self.say(json!({"kind":"candy_visit_ended","joined_seats":joined}));
                self.say(json!({"kind":"candy_van_left","x":van.x,"z":van.z}));
            }
        }
        self.candy_van = Some(van);
        changed
    }
    pub fn buy_candy(&mut self, seat: u8) -> Result<(), BoardError> {
        let van = self.candy_van.as_ref().ok_or(BoardError::WrongStep)?;
        if van.phase != Some(CandyVanPhase::Visit) || !van.joined.contains(&seat) {
            return Err(BoardError::WrongStep);
        }
        if van.bought.contains(&seat) {
            return Ok(());
        }
        let token = self
            .tokens
            .iter()
            .find(|t| t.seat == seat)
            .ok_or(BoardError::UnknownTarget)?;
        let run = van
            .runs
            .iter()
            .find(|r| r.view.seat == seat)
            .ok_or(BoardError::WrongStep)?;
        if (run.view.x - van.x).hypot(run.view.z - van.z) > BUY_RANGE {
            return Err(BoardError::OutOfReach);
        }
        if token.coins < PRIZE_COST {
            return Err(BoardError::TooDear);
        }
        let at = (van.x, van.z);
        self.coins(seat, -PRIZE_COST);
        let token = self.token(seat).unwrap();
        token.prizes += 1;
        let total = token.prizes;
        self.candy_van.as_mut().unwrap().bought.insert(seat);
        self.say(json!({"kind":"prize","source":"candy_van","seat":seat,"coins":PRIZE_COST,"prizes":total,"x":at.0,"z":at.1}));
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> Board {
        let mut b = Board::for_room(Map::Frozen, &[0, 1], 10, 17);
        for t in &mut b.tokens {
            t.coins = 60;
        }
        b
    }
    fn offer_done(b: &mut Board) {
        for _ in 0..101 {
            b.candy_van_tick(0.05);
        }
    }
    #[test]
    fn offer_requires_explicit_opt_in_and_no_visitors_resume_after_five_seconds() {
        let mut b = ready();
        assert!(b.candy_paused());
        assert_eq!(
            b.candy_van_view_for(Some(1)).unwrap().phase,
            CandyVanPhase::Offer
        );
        assert!(b.candy_van_view_for(Some(1)).unwrap().can_join);
        assert!(!b.candy_van_view_for(None).unwrap().can_join);
        assert_eq!(b.buy_candy(0), Err(BoardError::WrongStep));
        offer_done(&mut b);
        assert!(!b.candy_paused());
        assert!(b.candy_runs_view().is_empty());
        assert!(b.candy_van_view().is_none());
        assert_eq!(b.tokens[0].prizes, 0);
    }
    #[test]
    fn any_seat_can_join_without_money_and_personalization_tracks_each_visitor() {
        let mut b = ready();
        b.tokens[1].coins = 0;
        b.join_candy_visit(1).unwrap();
        b.join_candy_visit(1).unwrap();
        assert_eq!(b.candy_van_view().unwrap().joined_seats, vec![1]);
        assert!(!b.candy_van_view_for(Some(1)).unwrap().can_join);
        offer_done(&mut b);
        assert_eq!(b.candy_runs_view().len(), 1);
        assert_eq!(b.candy_runs_view()[0].seat, 1);
        assert_eq!(b.candy_van_view().unwrap().phase, CandyVanPhase::Visit);
        assert!(!b.candy_van_view_for(Some(1)).unwrap().can_buy);
        assert_eq!(b.candy_visit_input(0, 1., 0.), Err(BoardError::WrongStep));
        assert_eq!(
            b.candy_visit_input(1, f32::NAN, 0.),
            Err(BoardError::OutOfReach)
        );
    }
    #[test]
    fn off_turn_purchase_is_once_per_round_and_never_changes_prior_turn_walk_or_floating_candy() {
        let mut b = ready();
        b.start_walk(
            0,
            0,
            vec![0, 1],
            false,
            walking::AfterWalk::Step(TurnStep::Done),
        );
        let old_walk = b.walk_view().unwrap();
        let old_round = b.round;
        let old_prize = b.prize_at;
        b.join_candy_visit(1).unwrap();
        offer_done(&mut b);
        {
            let v = b.candy_van.as_mut().unwrap();
            let r = &mut v.runs[0];
            r.view.x = v.x;
            r.view.z = v.z;
        }
        assert!(b.candy_van_view_for(Some(1)).unwrap().can_buy);
        assert!(!b.candy_van_view_for(Some(0)).unwrap().can_buy);
        b.buy_candy(1).unwrap();
        for _ in 0..10 {
            b.buy_candy(1).unwrap();
        }
        assert_eq!(b.tokens[1].coins, 60 - PRIZE_COST);
        assert_eq!(b.tokens[1].prizes, 1);
        assert_eq!(b.round, old_round);
        assert_eq!(b.prize_at, old_prize);
        assert_eq!(b.walk_view().unwrap().id, old_walk.id);
        assert_eq!(b.walk_view().unwrap().elapsed, old_walk.elapsed);
        for _ in 0..301 {
            b.candy_van_tick(0.05);
        }
        assert!(!b.candy_paused());
        assert_eq!(b.walk_view().unwrap().elapsed, old_walk.elapsed);
        assert_eq!(b.step, TurnStep::Walking);
    }
    #[test]
    fn visitor_movement_is_bounded_has_input_ttl_and_original_actor_is_never_overwritten() {
        let mut b = ready();
        b.join_candy_visit(1).unwrap();
        let original = b.actual_spot_of(1);
        let old_tokens = serde_json::to_value(&b.tokens).unwrap();
        offer_done(&mut b);
        b.candy_visit_input(1, 100., 100.).unwrap();
        let old = b.candy_runs_view()[0].clone();
        b.candy_van_tick(100.);
        let now = b.candy_runs_view()[0].clone();
        assert!((now.x - old.x).hypot(now.z - old.z) <= RUN_SPEED * 0.05 + 0.0001);
        for _ in 0..20 {
            b.candy_van_tick(0.05);
        }
        let stopped = b.candy_runs_view()[0].clone();
        for _ in 0..30 {
            b.candy_van_tick(0.05);
        }
        let after = b.candy_runs_view()[0].clone();
        assert_eq!((stopped.x, stopped.z), (after.x, after.z));
        for _ in 0..301 {
            b.candy_van_tick(0.05);
        }
        assert_eq!(b.actual_spot_of(1), original);
        assert_eq!(serde_json::to_value(&b.tokens).unwrap(), old_tokens);
    }
    #[test]
    fn vanished_guests_and_late_inputs_cannot_gain_candy_or_extend_offer() {
        let mut b = ready();
        b.join_candy_visit(1).unwrap();
        for _ in 0..101 {
            b.candy_visit_tick(0.05, &[0]);
        }
        assert!(!b.candy_paused());
        assert!(b.candy_runs_view().is_empty());
        assert_eq!(b.candy_visit_input(1, 0., 1.), Err(BoardError::WrongStep));
        assert_eq!(b.buy_candy(1), Err(BoardError::WrongStep));
    }
    #[test]
    fn visits_remain_seeded_and_do_not_consume_other_game_randomness() {
        let mut present = 0;
        for seed in 1..=20 {
            let mut a = Board::on(Map::Frozen, &[0, 1], 10, seed);
            let mut b = Board::on(Map::Frozen, &[0, 1], 10, seed);
            a.round = 2;
            b.round = 2;
            a.restock_candy_van();
            b.restock_candy_van();
            assert_eq!(a.candy_paused(), b.candy_paused());
            present += usize::from(a.candy_paused());
            a.restock_candy_van();
            let ar: u64 = a.rng.random();
            let br: u64 = b.rng.random();
            assert_eq!(ar, br);
        }
        assert!(present > 0 && present < 20);
    }
}
