use party::city_heist::{CityConfig, CityGame};
fn main() {
    let seats: Vec<_> = (0..4).map(|seat| (seat, true)).collect();
    let mut game = CityGame::seeded(&seats, CityConfig::default(), true, 0);
    for tick in 0..60 * 365 {
        game.tick(1. / 60., &[], &[0, 1, 2, 3], |_, _| false);
        if tick % 1800 == 0 {
            let v = game.view_for(Some(1));
            println!(
                "time {:.0} score {:?} vault {:?} power {:?} wait {}",
                game.time, v.night_score, v.vault_progress, v.power_seconds, v.job_wait
            );
            for a in v.actors {
                println!(
                    " {} {:?} {:.1?}/{:.1?} jailed={} car={} holding={} carry={}",
                    a.seat, a.side, a.x, a.z, a.jailed, a.driving, a.acting, a.carrying
                );
            }
        }
        if game.resolved.is_some() {
            break;
        }
    }
}
