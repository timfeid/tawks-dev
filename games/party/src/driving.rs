//! A short, owned-turn car rental. The token stays on its safe home tile.
use super::*;
const SECONDS: f32 = 10.0;
const ARRIVAL: f32 = 0.7;
const MAX_SPEED: f32 = 7.0;
#[derive(Clone, Debug, Serialize)]
pub struct DriverView {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub speed: f32,
    pub remaining: f32,
    pub arriving: f32,
    pub home_x: f32,
    pub home_z: f32,
    pub home_space: u8,
}
pub(super) struct Driver {
    pub(super) view: DriverView,
    input: (f32, f32),
    input_age: f32,
    horn_age: f32,
    age: f32,
    after: TurnStep,
}
impl Board {
    /// The actual coast is the drop boundary. The old positioning shore
    /// margin is deliberately not used: cars can reach the edge and fall.
    fn drive_land(&self, point: (f32, f32)) -> bool {
        if !point.0.is_finite() || !point.1.is_finite() {
            return false;
        }
        static COASTS: OnceLock<[Vec<(f32, f32)>; 4]> = OnceLock::new();
        let all = COASTS.get_or_init(|| {
            Map::ALL.map(|map| {
                let design: Value = serde_json::from_str(map.source()).expect("driving coast");
                design["coast"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| Some((p[0].as_f64()? as f32, p[1].as_f64()? as f32)))
                    .collect()
            })
        });
        let coast = &all[Map::ALL.iter().position(|map| *map == self.map).unwrap()];
        if coast.len() < 3 {
            return false;
        }
        let mut inside = false;
        for index in 0..coast.len() {
            let (a, b) = (coast[index], coast[(index + 1) % coast.len()]);
            if (a.1 > point.1) != (b.1 > point.1)
                && point.0 < (b.0 - a.0) * (point.1 - a.1) / (b.1 - a.1) + a.0
            {
                inside = !inside;
            }
        }
        inside
    }
    pub fn driving_view(&self) -> Vec<DriverView> {
        self.driver
            .as_ref()
            .map(|d| vec![d.view.clone()])
            .unwrap_or_default()
    }
    pub fn end_drive(&mut self, reason: &str) {
        if let Some(d) = self.driver.take() {
            if matches!(self.step, TurnStep::Driving) {
                self.step = if matches!(reason, "collision" | "knockout") {
                    TurnStep::Done
                } else {
                    d.after.clone()
                };
            }
            self.say(json!({"kind":"drive_end","seat":d.view.seat,"x":d.view.x,"z":d.view.z,"home_space":d.view.home_space,"reason":reason}));
        }
    }
    pub fn has_driver_for(&self, seat: u8) -> bool {
        self.driver.as_ref().is_some_and(|d| d.view.seat == seat)
    }
    pub(super) fn begin_drive(&mut self, seat: u8) {
        let after = self.step.clone();
        let home = self.actual_spot_of(seat);
        let traffic = self.traffic_view().expect("car rentals require city");
        let mut candidates = Vec::new();
        for road in &traffic.roads {
            for segment in road.points.windows(2) {
                let (a, b) = (segment[0], segment[1]);
                let (dx, dz) = (b.0 - a.0, b.1 - a.1);
                let length = dx.hypot(dz);
                let along =
                    (((home.0 - a.0) * dx + (home.1 - a.1) * dz) / length).clamp(2., length - 2.);
                for offset in [-6., -3., 0., 3., 6.] {
                    let distance = (along + offset).clamp(2., length - 2.);
                    let p = (a.0 + dx / length * distance, a.1 + dz / length * distance);
                    let clearance = traffic
                        .cars
                        .iter()
                        .map(|c| (c.x - p.0).hypot(c.z - p.1))
                        .fold(f32::INFINITY, f32::min);
                    if clearance > 4.0 && self.drive_land(p) {
                        candidates.push(((p.0 - home.0).hypot(p.1 - home.1), p, dx.atan2(dz)));
                    }
                }
            }
        }
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (_, at, yaw) =
            candidates
                .first()
                .copied()
                .unwrap_or((0., (0., -16.), std::f32::consts::FRAC_PI_2));
        let home_space = self.space_of(seat);
        self.driver = Some(Driver {
            view: DriverView {
                seat,
                x: at.0,
                z: at.1,
                yaw,
                speed: 0.,
                remaining: SECONDS,
                arriving: ARRIVAL,
                home_x: home.0,
                home_z: home.1,
                home_space,
            },
            input: (0., 0.),
            input_age: 10.,
            horn_age: 1.,
            age: 0.,
            after,
        });
        self.step = TurnStep::Driving;
        self.say(
            json!({"kind":"drive_start","seat":seat,"x":at.0,"z":at.1,"yaw":yaw,"seconds":SECONDS}),
        );
    }
    pub fn drive_input(
        &mut self,
        seat: u8,
        throttle: f32,
        steer: f32,
        horn: bool,
    ) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if !matches!(self.step, TurnStep::Driving) {
            return Err(BoardError::WrongStep);
        }
        if !throttle.is_finite() || !steer.is_finite() {
            return Err(BoardError::OutOfReach);
        }
        let d = self.driver.as_mut().ok_or(BoardError::WrongStep)?;
        d.input = (throttle.clamp(-1., 1.), steer.clamp(-1., 1.));
        d.input_age = 0.;
        let honk = horn && d.horn_age >= 1.;
        let at = (d.view.x, d.view.z);
        if honk {
            d.horn_age = 0.;
            self.say(json!({"kind":"traffic_horn","car":100+seat,"seat":seat,"x":at.0,"z":at.1}));
        }
        Ok(())
    }
    pub fn drive_tick(&mut self, dt: f32, connected: &[u8]) -> bool {
        if !dt.is_finite() || dt <= 0. {
            return false;
        }
        let Some(mut d) = self.driver.take() else {
            return false;
        };
        let dt = dt.min(0.05);
        let seat = d.view.seat;
        d.age += dt;
        d.horn_age += dt;
        d.input_age += dt;
        d.view.remaining = (d.view.remaining - dt).max(0.);
        d.view.arriving = (ARRIVAL - d.age).max(0.);
        let mut reason = None;
        if seat != self.turn_seat() || !matches!(self.step, TurnStep::Driving) {
            reason = Some("turn");
        } else if !connected.contains(&seat) {
            reason = Some("disconnect");
        } else if d.view.remaining <= 0. {
            reason = Some("timeout");
        } else if self.tokens.iter().any(|t| t.seat == seat && t.resting > 0) {
            reason = Some("knockout");
        }
        if reason.is_none() && d.view.arriving <= 0. {
            let (throttle, steer) = if d.input_age <= 0.35 {
                d.input
            } else {
                (0., 0.)
            };
            if throttle.abs() > 0.02 {
                d.view.speed = (d.view.speed + throttle * 8. * dt).clamp(-2.5, MAX_SPEED);
            } else {
                d.view.speed *= (-5. * dt).exp();
            }
            d.view.yaw -= steer * 2.2 * dt * (d.view.speed.abs() / 2.).clamp(0.25, 1.);
            let old = (d.view.x, d.view.z);
            let next = (
                old.0 + d.view.yaw.sin() * d.view.speed * dt,
                old.1 + d.view.yaw.cos() * d.view.speed * dt,
            );
            d.view.x = next.0;
            d.view.z = next.1;
            if !self.drive_land(next) {
                reason = Some("fell");
            }
            if reason.is_none() && d.age > ARRIVAL + 0.8 {
                if let Some(car) = self.traffic_vehicle_collision(old, (d.view.x, d.view.z)) {
                    self.say(json!({"kind":"traffic_hit","car":car,"seat":seat,"x":d.view.x,"z":d.view.z,"speed":d.view.speed.abs()}));
                    self.hurt(seat, MAX_HEALTH);
                    reason = Some("collision");
                }
            }
            if reason.is_none() && d.view.speed.abs() > 1.5 {
                // Intentional player driving can hit ordinary parked pawns as
                // well as free actors; automatic traffic keeps its own mask.
                let victims: Vec<_> = self
                    .tokens
                    .iter()
                    .filter(|token| {
                        token.seat != seat
                            && token.resting == 0
                            && !self.layout[token.space as usize].under
                    })
                    .map(|token| token.seat)
                    .collect();
                let direction = (d.view.yaw.sin() * 0.7, d.view.yaw.cos() * 0.7);
                let tail = (old.0 - direction.0, old.1 - direction.1);
                let nose = (d.view.x + direction.0, d.view.z + direction.1);
                self.car_hit_pedestrians(seat, tail, nose, 0.95, d.view.speed);
                for other in victims {
                    if self.tokens.iter().any(|t| t.seat == other && t.resting > 0) {
                        continue;
                    }
                    let at = self.actual_spot_of(other);
                    let delta = (nose.0 - tail.0, nose.1 - tail.1);
                    let length = delta.0 * delta.0 + delta.1 * delta.1;
                    let t = (((at.0 - tail.0) * delta.0 + (at.1 - tail.1) * delta.1)
                        / length.max(0.00001))
                    .clamp(0., 1.);
                    if (at.0 - tail.0 - delta.0 * t).hypot(at.1 - tail.1 - delta.1 * t) <= 0.95 {
                        self.say(json!({"kind":"traffic_hit","car":255,"driver":seat,"seat":other,"x":at.0,"z":at.1,"speed":d.view.speed.abs()}));
                        self.hurt(other, MAX_HEALTH);
                        self.end_raid(other, "traffic");
                        if self.positioning.as_ref().is_some_and(|p| p.seat == other) {
                            self.finish_position();
                            if other == self.turn_seat() {
                                self.step = TurnStep::Done;
                            }
                        }
                    }
                }
            }
        }
        if let Some(reason) = reason {
            if matches!(self.step, TurnStep::Driving) {
                self.step = if matches!(reason, "collision" | "knockout") {
                    TurnStep::Done
                } else {
                    d.after.clone()
                };
            }
            self.say(json!({"kind":"drive_end","seat":seat,"x":d.view.x,"z":d.view.z,"home_space":d.view.home_space,"reason":reason}));
            true
        } else {
            self.driver = Some(d);
            false
        }
    }
    pub fn bot_drive_tick(&mut self) {
        let Some(d) = self.driver.as_ref() else {
            return;
        };
        let p = (d.view.x, d.view.z);
        let Some(road) = self
            .traffic_view()
            .and_then(|view| view.roads.into_iter().next())
        else {
            return;
        };
        let mut corners = road.points;
        if corners.first() == corners.last() {
            corners.pop();
        }
        if corners.len() < 2 {
            return;
        }
        let mut closest = 0;
        let mut distance = f32::INFINITY;
        for (i, at) in corners.iter().enumerate() {
            let dis = (at.0 - p.0).hypot(at.1 - p.1);
            if dis < distance {
                closest = i;
                distance = dis;
            }
        }
        let target = if distance < 3.2 {
            corners[(closest + 1) % corners.len()]
        } else {
            let ahead = (p.0 + d.view.yaw.sin() * 3., p.1 + d.view.yaw.cos() * 3.);
            corners
                .iter()
                .copied()
                .min_by(|a, b| {
                    (a.0 - ahead.0)
                        .hypot(a.1 - ahead.1)
                        .total_cmp(&(b.0 - ahead.0).hypot(b.1 - ahead.1))
                })
                .unwrap()
        };
        let yaw = (target.0 - p.0).atan2(target.1 - p.1);
        let diff = (yaw - d.view.yaw).sin().atan2((yaw - d.view.yaw).cos());
        let seat = d.view.seat;
        let _ = self.drive_input(
            seat,
            if diff.abs() > 1.0 { 0.2 } else { 0.6 },
            -diff.clamp(-1., 1.),
            false,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> Board {
        let mut b = Board::on(Map::Frozen, &[0, 1], 10, 7);
        b.timed_items = true;
        b.tokens[0].items = vec![Item::DriveCar];
        b
    }
    #[test]
    fn consumes_once_blocks_roll_and_returns_home() {
        let mut b = ready();
        let home = b.tokens[0].space;
        b.use_item(0, Item::DriveCar, None).unwrap();
        assert_eq!(b.step(), &TurnStep::Driving);
        assert!(b.tokens[0].items.is_empty());
        assert_eq!(
            b.use_item(0, Item::DriveCar, None),
            Err(BoardError::WrongStep)
        );
        assert_eq!(b.roll(0), Err(BoardError::WrongStep));
        assert_eq!(
            b.drive_input(1, 1., 0., false),
            Err(BoardError::NotYourTurn)
        );
        assert_eq!(
            b.drive_input(0, f32::NAN, 0., false),
            Err(BoardError::OutOfReach)
        );
        for _ in 0..220 {
            b.drive_tick(0.05, &[0, 1]);
        }
        assert!(b.driver.is_none());
        assert_eq!(b.step(), &TurnStep::Items);
        assert_eq!(b.tokens[0].space, home);
        assert_eq!(b.tokens[0].stats.items_used, 1);
    }
    #[test]
    fn driving_speed_is_bounded_and_disconnect_ends_session() {
        let mut b = ready();
        b.use_item(0, Item::DriveCar, None).unwrap();
        for _ in 0..140 {
            if b.driver.is_none() {
                break;
            }
            b.drive_input(0, 10., 3., false).unwrap();
            b.drive_tick(0.05, &[0, 1]);
            if let Some(d) = &b.driver {
                assert!(b.drive_land((d.view.x, d.view.z)));
                assert!(d.view.speed.abs() <= MAX_SPEED);
            }
        }
        // A long free-driving input can legitimately crash or fall now.
        assert!(b.driver.is_some() || matches!(b.step(), TurnStep::Done | TurnStep::Items));
        let mut disconnected = ready();
        disconnected.use_item(0, Item::DriveCar, None).unwrap();
        disconnected.drive_tick(0.05, &[1]);
        assert!(disconnected.driver.is_none());
        assert_eq!(disconnected.step(), &TurnStep::Items);
        assert!(
            disconnected
                .log
                .iter()
                .any(|event| event["kind"] == "drive_end" && event["reason"] == "disconnect")
        );
    }
    #[test]
    fn driver_crash_ends_the_turn_and_cannot_roll_or_reuse_the_item() {
        let mut b = ready();
        b.use_item(0, Item::DriveCar, None).unwrap();
        let car = b.traffic_view().unwrap().cars[0].clone();
        let d = b.driver.as_mut().unwrap();
        d.age = 2.;
        d.view.arriving = 0.;
        d.view.x = car.x;
        d.view.z = car.z;
        d.view.speed = 2.;
        b.drive_tick(0.01, &[0, 1]);
        assert!(b.driver.is_none());
        assert_eq!(b.tokens[0].resting, 1);
        assert_eq!(b.step(), &TurnStep::Done);
        assert_eq!(b.roll(0), Err(BoardError::WrongStep));
        assert_eq!(
            b.use_item(0, Item::DriveCar, None),
            Err(BoardError::WrongStep)
        );
        assert_eq!(b.tokens[0].stats.items_used, 1);
    }
    #[test]
    fn car_can_leave_the_road_and_drive_across_the_land() {
        let mut b = ready();
        b.use_item(0, Item::DriveCar, None).unwrap();
        let at = (-10..=10)
            .flat_map(|x| (-10..=10).map(move |z| (x as f32, z as f32)))
            .find(|p| b.drive_land(*p) && !b.on_traffic_road(*p, 1.))
            .expect("town has off-road land");
        let d = b.driver.as_mut().unwrap();
        d.age = 0.8;
        d.view.arriving = 0.;
        d.view.x = at.0;
        d.view.z = at.1;
        d.view.yaw = 0.;
        d.view.speed = 3.;
        b.drive_input(0, 1., 0., false).unwrap();
        b.drive_tick(0.05, &[0, 1]);
        let d = b
            .driver
            .as_ref()
            .expect("valid off-road travel keeps the rental");
        assert!(d.view.z > at.1 + 0.14);
        assert!(!b.on_traffic_road((d.view.x, d.view.z), 0.));
    }
    #[test]
    fn falling_off_loses_only_the_consumed_item_and_returns_to_the_home_turn() {
        let mut b = ready();
        let home = b.tokens[0].space;
        let health = b.tokens[0].health;
        let cash = b.tokens[0].coins;
        b.use_item(0, Item::DriveCar, None).unwrap();
        let edge = (1..2000)
            .map(|n| n as f32 * 0.1)
            .find(|x| !b.drive_land((*x, 0.)))
            .unwrap();
        let d = b.driver.as_mut().unwrap();
        d.age = 1.;
        d.view.arriving = 0.;
        d.view.x = edge - 0.11;
        d.view.z = 0.;
        d.view.yaw = std::f32::consts::FRAC_PI_2;
        d.view.speed = 7.;
        b.drive_input(0, 1., 0., false).unwrap();
        b.drive_tick(0.05, &[0, 1]);
        assert!(b.driver.is_none());
        assert!(b.tokens[0].items.is_empty());
        assert_eq!(b.tokens[0].stats.items_used, 1);
        assert_eq!(b.tokens[0].space, home);
        assert_eq!(b.tokens[0].health, health);
        assert_eq!(b.tokens[0].coins, cash);
        assert_eq!(b.step(), &TurnStep::Items);
        assert!(
            b.log
                .iter()
                .any(|event| event["kind"] == "drive_end" && event["reason"] == "fell")
        );
        b.drive_tick(0.05, &[0, 1]);
        assert_eq!(b.tokens[0].coins, cash);
        assert!(b.tokens[0].items.is_empty());
    }
    #[test]
    fn moving_car_can_run_over_an_ordinary_player_without_ending_the_drivers_turn() {
        let mut b = ready();
        b.use_item(0, Item::DriveCar, None).unwrap();
        let target = b.actual_spot_of(1);
        let d = b.driver.as_mut().unwrap();
        d.age = 0.8;
        d.view.arriving = 0.;
        d.view.x = target.0 - 1.;
        d.view.z = target.1;
        d.view.yaw = std::f32::consts::FRAC_PI_2;
        d.view.speed = 4.;
        b.drive_input(0, 1., 0., false).unwrap();
        b.drive_tick(0.05, &[0, 1]);
        assert_eq!(b.tokens[1].resting, 1);
        assert!(b.driver.is_some());
        assert_eq!(b.step(), &TurnStep::Driving);
        let hits = b
            .log
            .iter()
            .filter(|event| event["kind"] == "traffic_hit" && event["seat"] == 1)
            .count();
        assert_eq!(hits, 1);
        b.drive_tick(0.05, &[0, 1]);
        assert_eq!(
            b.log
                .iter()
                .filter(|event| event["kind"] == "traffic_hit" && event["seat"] == 1)
                .count(),
            1
        );
    }
    #[test]
    fn driving_hits_real_pedestrians_without_minting_cash() {
        let mut b = Board::for_room(Map::Frozen, &[0, 1], 10, 7);
        b.tokens[0].items = vec![Item::DriveCar];
        b.use_item(0, Item::DriveCar, None).unwrap();
        let target = b.pedestrian_views()[0].clone();
        let wallet = target.remaining_cash;
        let cash = b.tokens[0].coins;
        let d = b.driver.as_mut().unwrap();
        d.age = 0.8;
        d.view.arriving = 0.;
        d.view.x = target.x - 1.;
        d.view.z = target.z;
        d.view.yaw = std::f32::consts::FRAC_PI_2;
        d.view.speed = 4.;
        b.drive_input(0, 1., 0., false).unwrap();
        b.drive_tick(0.05, &[0, 1]);
        let npc = &b.pedestrian_views()[0];
        assert!(npc.knocked_down);
        assert!(!npc.walking);
        assert_eq!(npc.remaining_cash, wallet);
        assert_eq!(b.tokens[0].coins, cash);
        assert!(
            b.log
                .iter()
                .any(|e| e["kind"] == "npc_run_over" && e["npc"] == 0)
        );
    }
    #[test]
    fn ending_a_post_roll_car_session_cannot_grant_another_roll() {
        let mut b = ready();
        b.step = TurnStep::Done;
        b.begin_drive(0);
        b.end_drive("fell");
        assert_eq!(b.step(), &TurnStep::Done);
        assert_eq!(b.roll(0), Err(BoardError::WrongStep));
    }
    #[test]
    fn a_surface_car_cannot_run_over_a_player_in_the_tunnel_below_it() {
        let mut b = ready();
        let under = b
            .layout
            .iter()
            .find(|s| s.under)
            .expect("city has an underground passage")
            .id;
        b.tokens[1].space = under;
        b.use_item(0, Item::DriveCar, None).unwrap();
        let target = b.actual_spot_of(1);
        let d = b.driver.as_mut().unwrap();
        d.age = 0.8;
        d.view.arriving = 0.;
        d.view.x = target.0 - 1.;
        d.view.z = target.1;
        d.view.yaw = std::f32::consts::FRAC_PI_2;
        d.view.speed = 4.;
        b.drive_input(0, 1., 0., false).unwrap();
        b.drive_tick(0.05, &[0, 1]);
        assert_eq!(b.tokens[1].health, MAX_HEALTH);
        assert_eq!(b.tokens[1].resting, 0);
        assert!(
            !b.log
                .iter()
                .any(|e| e["kind"] == "traffic_hit" && e["seat"] == 1)
        );
    }
}
