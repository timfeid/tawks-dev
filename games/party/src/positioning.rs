//! Temporary, server-owned footing for aimed items. A route space is never
//! changed by free movement, so it cannot buy prizes or pay landing rewards.
use super::*;

pub const POSITION_SECONDS: f32 = 10.0;
pub const POSITION_SPEED: f32 = 4.0;
pub const POSITION_RADIUS: f32 = POSITION_SPEED*POSITION_SECONDS;
const INPUT_TTL: f32 = 0.35;

#[derive(Clone, Debug, Serialize)]
pub struct PositionView {
    pub seat: u8,
    pub item: Item,
    pub x: f32,
    pub z: f32,
    pub home_space: u8,
    pub home_x: f32,
    pub home_z: f32,
    pub remaining: f32,
    /// Movement ends when activated; the origin remains until the action resolves.
    pub active: bool,
    pub speed: f32,
    pub radius: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(map:Map,item:Item)->Board {
        let mut board=Board::on(map,&[0,1],5,77);
        board.timed_items=true;
        if item==Item::Crate {board.token(0).unwrap().carrying=true;}
        else if item==Item::Snowball {board.token(0).unwrap().snowball=true;}
        else {board.token(0).unwrap().items=vec![item];}
        board
    }
    fn select(board:&mut Board,item:Item) {
        match item {Item::Crate=>board.throw_crate(0),Item::Snowball=>board.throw_snowball(0),_=>board.use_item(0,item,None)}.unwrap();
    }

    #[test]
    fn positioning_reserves_every_aimed_item_consumes_once_and_returns_after_resolution() {
        for item in [Item::Bat,Item::Bomb,Item::Grapple,Item::Swap,Item::Curse,Item::Pickpocket,Item::Trap,Item::Crate,Item::Snowball] {
            let mut board=ready(Map::Island,item);
            let before=board.tokens[0].clone();
            select(&mut board,item);
            assert_eq!(board.step(),&TurnStep::Position{item});
            assert_eq!(board.tokens[0].items,before.items);
            assert_eq!(board.tokens[0].carrying,before.carrying);
            assert_eq!(board.tokens[0].snowball,before.snowball);
            assert_eq!(board.tokens[0].stats.items_used,0);
            assert_eq!(board.activate_item(1),Err(BoardError::NotYourTurn));
            assert_eq!(board.lob(0,0.0,0.5,0.5),Err(BoardError::WrongStep));
            assert!(!board.end_turn(),"cannot hand a reserved item to the next player");
            board.activate_item(0).unwrap();
            assert!(board.position_view().unwrap().active);
            assert!(board.tokens[0].items.is_empty());
            assert!(!board.tokens[0].carrying && !board.tokens[0].snowball);
            assert_eq!(board.activate_item(0),Err(BoardError::WrongStep));
            if item==Item::Bat {board.swing(0,0.0,0.5).unwrap();}
            else {board.lob(0,0.0,0.5,0.3).unwrap();}
            assert!(board.position_view().is_none());
            assert_eq!(board.step(),&TurnStep::Items);
        }
    }

    #[test]
    fn positioning_cancel_is_a_true_reservation_refund_and_self_checks_stay_immediate() {
        let mut board=ready(Map::Island,Item::Grapple);
        for _ in 0..3 {
            select(&mut board,Item::Grapple);
            board.position_input(0,1.0,0.0).unwrap();
            board.position_tick(0.05);
            board.cancel_item(0).unwrap();
            assert!(board.position_view().is_none());
            assert_eq!(board.tokens[0].items,vec![Item::Grapple]);
            assert_eq!(board.tokens[0].stats.items_used,0);
        }
        assert_eq!(board.cancel_item(0),Err(BoardError::WrongStep));
        board.token(0).unwrap().items=vec![Item::Shield];
        board.use_item(0,Item::Shield,None).unwrap();
        assert!(matches!(board.step(),TurnStep::Check{item:Item::Shield,..}));
        assert!(board.position_view().is_none());
    }

    #[test]
    fn positioning_input_is_bounded_authoritative_and_never_changes_board_economy() {
        for map in Map::ALL {
            let mut board=ready(map,Item::Grapple);
            let before=board.tokens[0].clone();
            select(&mut board,Item::Grapple);
            assert_eq!(board.position_input(1,1.0,0.0),Err(BoardError::NotYourTurn));
            assert_eq!(board.position_input(0,f32::NAN,0.0),Err(BoardError::OutOfReach));
            assert_eq!(board.position_input(0,0.0,f32::INFINITY),Err(BoardError::OutOfReach));
            let home=board.position_view().unwrap();
            board.position_input(0,100.0,100.0).unwrap();
            board.position_tick(100.0);
            let one=board.position_view().unwrap();
            assert!((one.x-home.x).hypot(one.z-home.z)<=POSITION_SPEED*0.05+0.0001,"large client time cannot teleport");
            for tick in 0..140 {
                let angle=tick as f32*0.1;
                board.position_input(0,angle.cos()*100.0,angle.sin()*100.0).unwrap();
                let old=board.position_view().unwrap();
                board.position_tick(0.05);
                let position=board.position_view().unwrap();
                assert!((position.x-old.x).hypot(position.z-old.z)<=POSITION_SPEED*0.05+0.0002);
                assert!((position.x-position.home_x).hypot(position.z-position.home_z)<=POSITION_RADIUS+0.0002);
                assert!(board.position_ground((position.x,position.z),position.home_space));
            }
            for _ in 0..10 {board.position_tick(0.05);}
            let stopped=board.position_view().unwrap();
            board.position_tick(0.05);
            let still=board.position_view().unwrap();
            assert_eq!((stopped.x,stopped.z),(still.x,still.z),"lost input stops motion");
            assert_eq!(board.tokens[0].space,before.space);
            assert_eq!(board.tokens[0].coins,before.coins);
            assert_eq!(board.tokens[0].prizes,before.prizes);
            assert_eq!(board.tokens[0].health,before.health);
            assert!(!board.position_ground((9999.0,9999.0),0));
        }
    }

    #[test]
    fn positioning_ten_second_timeout_activates_once_without_starting_the_fuse_early() {
        let mut board=ready(Map::Island,Item::Bomb);
        select(&mut board,Item::Bomb);
        for _ in 0..599 {board.position_tick(1.0/60.0);}
        assert!(matches!(board.step(),TurnStep::Position{..}));
        assert_eq!(board.tokens[0].stats.items_used,0);
        board.position_tick(1.0/60.0);
        assert!(matches!(board.step(),TurnStep::Lob{item:Item::Bomb,..}));
        assert_eq!(board.tokens[0].stats.items_used,1);
        for _ in 0..100 {board.position_tick(1.0/60.0);}
        assert_eq!(board.tokens[0].stats.items_used,1);
        assert_eq!(board.log.iter().filter(|entry|entry["kind"]=="position"&&entry["phase"]=="activate").count(),1);
        assert_eq!(board.position_input(0,1.0,0.0),Err(BoardError::WrongStep));
        board.fuse_out(0);
        assert!(board.position_view().is_none());
        assert_eq!(board.step(),&TurnStep::Items);
    }

    #[test]
    fn positioning_throw_origin_and_in_hand_blast_use_actual_feet_then_reset() {
        let mut board=ready(Map::Island,Item::Bomb);
        let home=board.spot_of(0);
        select(&mut board,Item::Bomb);
        for _ in 0..40 {board.position_input(0,1.0,0.0).unwrap();board.position_tick(0.05);}
        let position=board.position_view().unwrap();
        assert!((position.x-home.0).hypot(position.z-home.1)>3.0);
        board.activate_item(0).unwrap();
        board.calm_for_test();
        board.lob(0,0.0,0.0,0.0).unwrap();
        let blast=board.log.iter().rev().find(|entry|entry["item"]=="bomb"&&entry["x"].is_number()).unwrap();
        assert!((blast["x"].as_f64().unwrap()-position.x as f64).abs()<0.0001);
        assert!((blast["z"].as_f64().unwrap()-position.z as f64).abs()<0.0001);
        assert_eq!(blast["origin_x"],json!(position.x));
        assert_eq!(blast["origin_z"],json!(position.z));
        assert_eq!(board.tokens[0].health,MAX_HEALTH-1,"self blast uses free position, not original tile");
        assert!(board.position_view().is_none());
        assert_eq!(board.spot_of(0),home);
    }

    #[test]
    fn positioning_bot_closes_distance_for_a_bat_and_uses_the_same_phase() {
        let mut board=ready(Map::Island,Item::Bat);
        board.place_for_test(1,3,30);
        let before=board.spot_of(0);
        let target=board.item_spot_of(1);
        select(&mut board,Item::Bat);
        for _ in 0..250 {
            board.bot_position_tick();
            board.position_tick(1.0/60.0);
            if !matches!(board.step(),TurnStep::Position{..}) {break;}
        }
        let position=board.position_view().unwrap();
        assert!(position.active);
        assert!((position.x-target.0).hypot(position.z-target.1)<(before.0-target.0).hypot(before.1-target.1));
        assert!((position.x-position.home_x).hypot(position.z-position.home_z)>1.0,"bot actually repositions");
        let BotMove::Swing{yaw,stop}=board.bot_move() else {panic!("bat is active")};
        board.swing(0,yaw,stop).unwrap();
        assert!(board.log.iter().any(|entry|entry["item"]=="bat"&&entry["target"]==1));
        assert!(board.position_view().is_none());
        assert_eq!(board.tokens[0].space,0);
    }

    #[test]
    fn positioning_room_disconnect_releases_reserved_or_active_item_without_an_offset() {
        use crate::room::Room;
        for activate in [false,true] {
            let mut room=Room::new("disconnect-position",17);
            room.connect("dev","Dev","").unwrap();
            room.dev_tools=true;
            room.start_sandbox("dev").unwrap();room.finish_candy_offer_for_test();
            let seat=room.seat_of("dev").unwrap();
            room.board.as_mut().unwrap().dev_give(seat,Item::Bomb);
            room.use_item("dev",Item::Bomb,None,None).unwrap();
            room.position_input("dev",1.0,0.0).unwrap();
            room.step();
            if activate {room.activate_item("dev").unwrap();}
            room.disconnect("dev");
            let board=room.board.as_ref().unwrap();
            assert!(board.position_view().is_none());
            assert_eq!(board.step(),&TurnStep::Items);
            let token=board.tokens.iter().find(|token|token.seat==seat).unwrap();
            assert_eq!(token.items.contains(&Item::Bomb),!activate);
        }
    }

    #[test]
    fn positioning_room_grants_the_full_ten_seconds_then_a_fresh_action_fuse() {
        use crate::room::Room;
        let mut room=Room::new("position-countdown",19);
        room.connect("dev","Dev","").unwrap();
        room.dev_tools=true;
        room.start_sandbox("dev").unwrap();room.finish_candy_offer_for_test();
        let seat=room.seat_of("dev").unwrap();
        room.board.as_mut().unwrap().dev_give(seat,Item::Bomb);
        room.use_item("dev",Item::Bomb,None,None).unwrap();
        for _ in 0..599 {room.step();}
        assert!(matches!(room.board.as_ref().unwrap().step(),TurnStep::Position{..}),"positioning has no eight-second item fuse");
        room.step();
        assert!(matches!(room.board.as_ref().unwrap().step(),TurnStep::Lob{..}));
        for _ in 0..478 {room.step();}
        assert!(matches!(room.board.as_ref().unwrap().step(),TurnStep::Lob{..}),"activation starts a fresh fuse");
        room.step();
        assert_eq!(room.board.as_ref().unwrap().step(),&TurnStep::Items);
        assert!(room.board.as_ref().unwrap().position_view().is_none());
    }

    #[test]
    fn positioning_can_move_past_the_old_eight_unit_leash_on_valid_land() {
        let mut board=ready(Map::Island,Item::Grapple);select(&mut board,Item::Grapple);
        let home=board.position_view().unwrap();
        for _ in 0..180 {board.position_input(0,1.,0.).unwrap();board.position_tick(1./60.);}
        let moved=board.position_view().unwrap();
        assert!((moved.x-home.x).hypot(moved.z-home.z)>9.);
        assert!(board.position_ground((moved.x,moved.z),moved.home_space));
        assert_eq!(board.tokens[0].space,home.home_space);
    }
}

pub(super) struct Position {
    pub(super) seat: u8,
    pub(super) item: Item,
    pub(super) x: f32,
    pub(super) z: f32,
    home_space: u8,
    home: (f32, f32),
    remaining: f32,
    active: bool,
    input: (f32, f32),
    input_age: f32,
}

pub(super) fn aimed(item: Item) -> bool {
    matches!(item, Item::Bat | Item::Bomb | Item::Grapple | Item::Swap | Item::Curse | Item::Pickpocket | Item::Trap | Item::Crate | Item::Snowball | Item::Slapshot)
}

#[derive(Deserialize)]
struct Land {
    coast: Vec<(f32, f32)>,
    #[serde(default)]
    land: Vec<Feature>,
}
#[derive(Deserialize)]
struct Feature {
    kind: String,
    #[serde(default)] x: f32,
    #[serde(default)] z: f32,
    #[serde(default)] r: f32,
    #[serde(default)] frozen: bool,
    #[serde(default)] points: Vec<(f32, f32)>,
}
fn land(map: Map) -> &'static Land {
    static ISLAND: OnceLock<Land> = OnceLock::new();
    static VOLCANO: OnceLock<Land> = OnceLock::new();
    static SHIPWRECK: OnceLock<Land> = OnceLock::new();
    static FROZEN: OnceLock<Land> = OnceLock::new();
    let cache = match map { Map::Island => &ISLAND, Map::Volcano => &VOLCANO, Map::Shipwreck => &SHIPWRECK, Map::Frozen => &FROZEN };
    cache.get_or_init(|| serde_json::from_str(map.source()).expect("positioning land design"))
}
fn segment_distance(point: (f32, f32), from: (f32, f32), to: (f32, f32)) -> f32 {
    let delta = (to.0-from.0, to.1-from.1);
    let t = (((point.0-from.0)*delta.0+(point.1-from.1)*delta.1)/(delta.0*delta.0+delta.1*delta.1).max(0.0001)).clamp(0.0,1.0);
    (point.0-from.0-delta.0*t).hypot(point.1-from.1-delta.1*t)
}

impl Board {
    pub(super) fn begin_position(&mut self, seat: u8, item: Item) {
        let home = self.rendered_spot_of(seat);
        self.positioning = Some(Position {seat,item,x:home.0,z:home.1,home_space:self.space_of(seat),home,remaining:POSITION_SECONDS,active:false,input:(0.0,0.0),input_age:INPUT_TTL});
        self.step = TurnStep::Position {item};
        self.say(json!({"kind":"position","phase":"begin","seat":seat,"item":item,"x":home.0,"z":home.1,"seconds":POSITION_SECONDS}));
    }

    pub(super) fn position_view(&self) -> Option<PositionView> {
        let position = self.positioning.as_ref()?;
        Some(PositionView {seat:position.seat,item:position.item,x:position.x,z:position.z,home_space:position.home_space,home_x:position.home.0,home_z:position.home.1,remaining:position.remaining.max(0.0),active:position.active,speed:POSITION_SPEED,radius:self.movement_extent()})
    }

    pub fn position_input(&mut self, seat: u8, x: f32, z: f32) -> Result<(), BoardError> {
        if seat != self.turn_seat() { return Err(BoardError::NotYourTurn); }
        if !matches!(self.step, TurnStep::Position {..}) { return Err(BoardError::WrongStep); }
        if !x.is_finite() || !z.is_finite() { return Err(BoardError::OutOfReach); }
        let position = self.positioning.as_mut().ok_or(BoardError::WrongStep)?;
        let length = x.hypot(z).max(1.0);
        position.input = (x/length,z/length);
        position.input_age = 0.0;
        Ok(())
    }

    /// The item remains in the bag while reserved. Only activation consumes it,
    /// through exactly the ordinary item path, with the positioning gate off.
    pub fn activate_item(&mut self, seat: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() { return Err(BoardError::NotYourTurn); }
        if !matches!(self.step, TurnStep::Position {..}) { return Err(BoardError::WrongStep); }
        let position=self.positioning.as_ref().filter(|position|position.seat==seat&&!position.active).ok_or(BoardError::WrongStep)?;
        let item=position.item;
        let waited=POSITION_SECONDS-position.remaining;
        self.step = TurnStep::Items;
        let timed = std::mem::replace(&mut self.timed_items, false);
        let result = match item {
            Item::Crate => self.throw_crate(seat),
            Item::Snowball => self.throw_snowball(seat),
            _ => self.use_item_with(seat,item,None,None),
        };
        self.timed_items = timed;
        if result.is_ok() {
            if let Some(position) = self.positioning.as_mut() {position.active=true;position.remaining=0.0;position.input=(0.0,0.0);}
            if let Some(position)=self.positioning.as_ref() {
                self.say(json!({"kind":"position","phase":"activate","seat":seat,"item":item,"x":position.x,"z":position.z,"distance":(position.x-position.home.0).hypot(position.z-position.home.1),"waited":waited,"timed_out":waited>=POSITION_SECONDS-0.001}));
            }
        } else { self.positioning=None; }
        result
    }

    pub fn cancel_item(&mut self, seat: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() { return Err(BoardError::NotYourTurn); }
        if !matches!(self.step, TurnStep::Position {..}) { return Err(BoardError::WrongStep); }
        self.positioning = None;
        self.step = TurnStep::Items;
        self.say(json!({"kind":"position","phase":"cancel","seat":seat}));
        Ok(())
    }

    pub(super) fn finish_position(&mut self) { self.positioning=None; }

    pub fn abort_position(&mut self, seat: u8) {
        if self.positioning.as_ref().is_none_or(|position|position.seat!=seat) {return;}
        if matches!(self.step,TurnStep::Position {..}) {let _=self.cancel_item(seat);}
        else {self.fuse_out(seat);}
    }

    /// Paths/decks/bridges stay traversable even over water. Away from a road,
    /// keep clear of coast, open pools, rivers and the volcano's crater.
    pub(super) fn position_ground(&self, point: (f32,f32), home_space: u8) -> bool {
        let home = &self.layout[home_space as usize];
        let on_path = self.layout.iter().any(|space| {
            if home.under && space.path!=home.path { return false; }
            if point.0.hypot(point.1)>1000.0 { return false; }
            if (point.0-space.x).hypot(point.1-space.z)<=1.35 {return true;}
            space.next.iter().any(|next| {
                let next=&self.layout[*next as usize];
                space.link!=Some(next.id) && (!space.gated || self.gates_open())
                    && segment_distance(point,(space.x,space.z),(next.x,next.z))<=1.2
            })
        });
        if on_path {return true;}
        if home.under {return false;}
        let design=land(self.map);
        let mut inside=false;
        let mut shore=f32::INFINITY;
        for index in 0..design.coast.len() {
            let (a,b)=(design.coast[index],design.coast[(index+1)%design.coast.len()]);
            if (a.1>point.1)!=(b.1>point.1) && point.0<(b.0-a.0)*(point.1-a.1)/(b.1-a.1)+a.0 {inside=!inside;}
            shore=shore.min(segment_distance(point,a,b));
        }
        inside && shore>=4.5 && !design.land.iter().any(|feature|match feature.kind.as_str() {
            "pool" if !feature.frozen => (point.0-feature.x).hypot(point.1-feature.z)<feature.r+0.6,
            "crater" => (point.0-feature.x).hypot(point.1-feature.z)<feature.r*0.7,
            "river" => feature.points.windows(2).any(|line|segment_distance(point,line[0],line[1])<feature.r+0.6),
            _=>false,
        })
    }

    pub fn position_tick(&mut self, dt: f32) -> bool {
        if !dt.is_finite() || dt<=0.0 {return false;}
        let Some(mut position)=self.positioning.take() else {return false;};
        if position.active {
            if matches!(self.step,TurnStep::Lob {..}|TurnStep::Swing {..}) {self.positioning=Some(position);}
            return false;
        }
        let dt=dt.min(0.05);
        let before=(position.x,position.z);
        position.remaining=(position.remaining-dt).max(0.0);
        position.input_age+=dt;
        if position.input_age<=INPUT_TTL {
            (position.x,position.z)=self.bounded_position_step((position.x,position.z),position.home,position.home_space,position.input,dt);
        }
        let seat=position.seat;
        let expired=position.remaining<=0.0001;
        let moved=before!=(position.x,position.z);
        if moved {if let Some(token)=self.token(seat) {token.facing=Some((position.x-before.0).atan2(position.z-before.1));}}
        self.positioning=Some(position);
        if expired {let _=self.activate_item(seat);}
        moved||expired
    }

    pub(super) fn bounded_position_step(&self,point:(f32,f32),home:(f32,f32),home_space:u8,input:(f32,f32),dt:f32)->(f32,f32) {
        let _=home;
        let next=(point.0+input.0*POSITION_SPEED*dt,point.1+input.1*POSITION_SPEED*dt);
        let valid=|at:(f32,f32)| self.position_ground(at,home_space);
        if valid(next) {next}
        else if valid((next.0,point.1)) {(next.0,point.1)}
        else if valid((point.0,next.1)) {(point.0,next.1)}
        else {point}
    }

    pub(super) fn movement_extent(&self)->f32 {
        let coast=&land(self.map).coast;
        let bounds=coast.iter().fold((f32::INFINITY,f32::NEG_INFINITY,f32::INFINITY,f32::NEG_INFINITY),|b,p|(b.0.min(p.0),b.1.max(p.0),b.2.min(p.1),b.3.max(p.1)));
        (bounds.1-bounds.0).hypot(bounds.3-bounds.2)
    }

    /// Bots take the same bounded movement path as players, approaching a rival
    /// until the item is useful rather than walking away or burning ten seconds.
    pub fn bot_position_tick(&mut self) {
        let Some(position)=self.positioning.as_ref().filter(|position|!position.active) else {return;};
        let seat=position.seat;
        let item=position.item;
        let from=(position.x,position.z);
        let target=self.tokens.iter().filter(|token|self.rival(seat,token.seat))
            .filter(|token| item!=Item::Pickpocket || !token.items.is_empty())
            .map(|token|(token,self.rendered_spot_of(token.seat)))
            .filter(|(_,at)|(at.0-position.x).hypot(at.1-position.z)<=BOT_THROW+POSITION_SPEED*position.remaining)
            .max_by_key(|(token,_)|(token.prizes,token.coins)).map(|(_,at)|at);
        let desired=if item==Item::Bat {BAT_RANGE*0.6} else if item==Item::Bomb {BOMB_BLAST_RADIUS+2.0} else {9.0};
        let delta=target.map(|at|(at.0-from.0,at.1-from.1)).unwrap_or((0.0,0.0));
        let distance=delta.0.hypot(delta.1);
        let elapsed=POSITION_SECONDS-position.remaining;
        if elapsed>=0.5 && (target.is_none() || distance<=desired+0.4 || position.remaining<=0.1) {
            let _=self.activate_item(seat);
        } else if distance>desired {let _=self.position_input(seat,delta.0/distance,delta.1/distance);}
    }

    pub(super) fn item_spot_of(&self,seat:u8)->(f32,f32) {
        if self.timed_items {self.rendered_spot_of(seat)} else {self.spot_of(seat)}
    }
}
