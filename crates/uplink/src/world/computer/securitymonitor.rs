//! Port of source/world/computer/securitymonitor.h + source/world/computer/securitymonitor.cpp
//!
//! The C++ class is a purely static object representing the security system currently being
//! attacked by the player (never serialised, only one in existence). PORT: the statics become the
//! fields of this struct, owned by the context as `Ctx::security_monitor` (`g.security_monitor`).

use crate::ctx::Ctx;
use crate::world::computer::computer::Computer;
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::world::World;

#[derive(Debug, Clone, Default)]
pub struct SecurityMonitor {
    /// 0 = unused, 1 = enabled, 2 = monitering
    pub status: i32,
    /// One bool for each system, true if monitored
    pub currentmonitor: Vec<bool>,
    /// Date of next system monitor beginning
    pub nextmonitor_date: Date,
    /// Index of next system to monitor
    pub nextmonitor_index: i32,
}

/// Takes the monitor state out of the context, so the logic can call into `g` freely.
fn take_current(g: &mut Ctx) -> SecurityMonitor {
    std::mem::take(&mut g.security_monitor)
}

fn put_current(g: &mut Ctx, m: SecurityMonitor) {
    g.security_monitor = m;
}

/// `game->GetWorld()->GetVLocation(game->GetWorld()->GetPlayer()->remotehost)->GetComputer()`
fn remote_computer<'a>(world: &'a World, remotehost: &str) -> &'a Computer {
    let vl = world
        .get_vlocation(remotehost)
        .expect("UplinkAssert(vl)");
    vl.get_computer(world).expect("UplinkAssert(comp)")
}

impl SecurityMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call when player connects somewhere
    pub fn begin_attack(g: &mut Ctx) {
        // Look up the computer system the player is connected to
        let remotehost = g.world().get_player().remotehost.clone();
        let num_systems = remote_computer(g.world(), &remotehost)
            .security
            .num_systems();

        // Enable monitering if there are systems to be monitered

        if num_systems > 0 {
            let mut m = take_current(g);

            m.currentmonitor = vec![false; num_systems as usize];

            m.nextmonitor_date.set_date_from(&g.world().date);
            m.nextmonitor_date.advance_second(30 / num_systems);

            m.nextmonitor_index = 0;

            m.status = 1;

            put_current(g, m);
        }
    }

    /// Call when player disconnects
    pub fn end_attack(&mut self) {
        self.currentmonitor.clear();
        self.status = 0;
    }

    pub fn is_monitored(&self, index: i32) -> bool {
        self.status == 2
            && index >= 0
            && self
                .currentmonitor
                .get(index as usize)
                .copied()
                .unwrap_or(false)
    }

    /// Call frequently
    pub fn update(g: &mut Ctx) {
        let mut m = take_current(g);

        match m.status {
            0 => {
                // Not enabled
            }

            1 => {
                // Enabled, waiting to begin monitoring

                if g.world().date.after(&m.nextmonitor_date) {
                    let remotehost = g.world().get_player().remotehost.clone();
                    let (monitor_running, num_systems) = {
                        let comp = remote_computer(g.world(), &remotehost);
                        (
                            comp.security.is_running_monitor(),
                            comp.security.num_systems(),
                        )
                    };

                    if monitor_running {
                        // Time up - begin monitering the next system

                        m.currentmonitor[m.nextmonitor_index as usize] = true;

                        // You'll be rumbled in ~ 4 seconds
                        m.nextmonitor_date.set_date_from(&g.world().date);
                        m.nextmonitor_date.advance_second(4);

                        m.status = 2;
                    } else {
                        m.nextmonitor_date.set_date_from(&g.world().date);
                        m.nextmonitor_date.advance_second(30 / num_systems);
                    }
                }
            }

            2 => {
                // Monitering

                if g.world().date.after(&m.nextmonitor_date) {
                    let remotehost = g.world().get_player().remotehost.clone();
                    let monitor_running = remote_computer(g.world(), &remotehost)
                        .security
                        .is_running_monitor();

                    if monitor_running {
                        // Time up - if player has jimmied this system then he
                        // is now caught. One trace per caught system, as in C++.

                        let caught = {
                            let comp = remote_computer(g.world(), &remotehost);
                            let mut caught = 0;
                            for i in 0..m.currentmonitor.len() {
                                if m.currentmonitor[i] {
                                    let ss = comp
                                        .security
                                        .get_system(i as i32)
                                        .expect("UplinkAssert(ss)");
                                    if ss.bypassed || !ss.enabled {
                                        caught += 1;
                                    }
                                }
                            }
                            caught
                        };

                        for _ in 0..caught {
                            Connection::begin_trace(g, "PLAYER");
                        }
                    }

                    // Reset and wait for a few seconds before analysing the next system

                    let comp = remote_computer(g.world(), &remotehost);

                    m.nextmonitor_index += 1;
                    if comp.security.get_system(m.nextmonitor_index).is_none() {
                        m.nextmonitor_index = 0;
                    }

                    for slot in m.currentmonitor.iter_mut() {
                        *slot = false;
                    }

                    m.nextmonitor_date.set_date_from(&g.world().date);
                    m.nextmonitor_date
                        .advance_second(30 / comp.security.num_systems());

                    m.status = 1;
                }
            }

            _ => {
                panic!("Unrecognised status");
            }
        }

        put_current(g, m);
    }
}
