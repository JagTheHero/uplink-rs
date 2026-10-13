//! Port of source/world/date.h + source/world/date.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::game::{GAMESPEED_FAST, GAMESPEED_MEGAFAST, GAMESPEED_NORMAL, GAMESPEED_OHMYGOD};

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The date and time in the Uplink world.
// PORT: the C++ fields were `protected` and read through getters; they are `pub` here so the
// date can be built/compared directly, but prefer the getters/`advance_*` to keep values rolled over.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Date {
    pub second: i32,
    pub minute: i32,
    pub hour: i32,
    pub day: i32,
    pub month: i32,
    pub year: i32,

    /// `EclGetAccurateTime()` of the last time this date was advanced (not saved in C++ either).
    #[serde(skip)]
    pub previousupdate: i32,

    /// Should the time be advanced each update? (only `World::date` is ever activated)
    pub updateme: bool,
}

impl Default for Date {
    fn default() -> Self {
        Date {
            second: 1,
            minute: 1,
            hour: 1,
            day: 1,
            month: 1,
            year: 1000,
            previousupdate: 0,
            updateme: false,
        }
    }
}

impl Date {
    pub fn new(
        newsecond: i32,
        newminute: i32,
        newhour: i32,
        newday: i32,
        newmonth: i32,
        newyear: i32,
    ) -> Self {
        let mut d = Date {
            updateme: false,
            previousupdate: 0,
            ..Date::default()
        };
        d.set_date(newsecond, newminute, newhour, newday, newmonth, newyear);
        d
    }

    /// `SetDate(second, minute, hour, day, month, year)`; rolls over any dodgy values.
    pub fn set_date(
        &mut self,
        newsecond: i32,
        newminute: i32,
        newhour: i32,
        newday: i32,
        newmonth: i32,
        newyear: i32,
    ) {
        self.second = newsecond;
        self.minute = newminute;
        self.hour = newhour;
        self.day = newday;
        self.month = newmonth;
        self.year = newyear;
        self.advance_second(0);
    }

    /// `SetDate(Date *copydate)` (the overload; Rust has no overloading).
    pub fn set_date_from(&mut self, copydate: &Date) {
        self.set_date(
            copydate.get_second(),
            copydate.get_minute(),
            copydate.get_hour(),
            copydate.get_day(),
            copydate.get_month(),
            copydate.get_year(),
        );
    }

    /// Update me
    pub fn activate(&mut self) {
        self.updateme = true;
    }

    /// Don't update me
    pub fn deactivate(&mut self) {
        self.updateme = false;
    }

    /// true if this.date < date
    pub fn before(&self, date: &Date) -> bool {
        (
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
        ) < (
            date.year,
            date.month,
            date.day,
            date.hour,
            date.minute,
            date.second,
        )
    }

    /// true if this.date > date
    pub fn after(&self, date: &Date) -> bool {
        (
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
        ) > (
            date.year,
            date.month,
            date.day,
            date.hour,
            date.minute,
            date.second,
        )
    }

    /// true if this.date = date
    pub fn equal(&self, date: &Date) -> bool {
        (
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
        ) == (
            date.year,
            date.month,
            date.day,
            date.hour,
            date.minute,
            date.second,
        )
    }

    // The advance functions can be used to subtract time as well; they roll over into the next unit.
    pub fn advance_second(&mut self, n: i32) {
        self.second += n;

        if self.second > 59 {
            let numminutes = 1 + (self.second - 60) / 60;
            self.second %= 60;
            self.advance_minute(numminutes);
        }

        if self.second < 0 {
            let numminutes = 1 + (self.second * -1 - 1) / 60;
            self.second += 60 * numminutes;
            self.advance_minute(numminutes * -1);
        }
    }

    pub fn advance_minute(&mut self, n: i32) {
        self.minute += n;

        if self.minute > 59 {
            let numhours = 1 + (self.minute - 60) / 60;
            self.minute %= 60;
            self.advance_hour(numhours);
        }

        if self.minute < 0 {
            let numhours = 1 + (self.minute * -1 - 1) / 60;
            self.minute += 60 * numhours;
            self.advance_hour(numhours * -1);
        }
    }

    pub fn advance_hour(&mut self, n: i32) {
        self.hour += n;

        if self.hour > 23 {
            let numdays = 1 + (self.hour - 24) / 24;
            self.hour %= 24;
            self.advance_day(numdays);
        }

        if self.hour < 0 {
            let numdays = 1 + (self.hour * -1 - 1) / 24;
            self.hour += 24 * numdays;
            self.advance_day(numdays * -1);
        }
    }

    // Months are 30 days long in Uplink.
    pub fn advance_day(&mut self, n: i32) {
        self.day += n;

        if self.day > 30 {
            let nummonths = 1 + (self.day - 30) / 30;
            self.day %= 30;
            self.advance_month(nummonths);
        }

        if self.day < 1 {
            let nummonths = 1 + (self.day * -1) / 30;
            self.day += 30 * nummonths;
            self.advance_month(nummonths * -1);
        }
    }

    pub fn advance_month(&mut self, n: i32) {
        self.month += n;

        if self.month > 12 {
            let numyears = 1 + (self.month - 12) / 12;
            self.month %= 12;
            self.advance_year(numyears);
        }

        if self.month < 1 {
            let numyears = 1 + (self.month * -1) / 12;
            self.month += 12 * numyears;
            self.advance_year(numyears * -1);
        }
    }

    pub fn advance_year(&mut self, n: i32) {
        self.year += n;
    }

    pub fn get_second(&self) -> i32 {
        self.second
    }

    pub fn get_minute(&self) -> i32 {
        self.minute
    }

    pub fn get_hour(&self) -> i32 {
        self.hour
    }

    pub fn get_day(&self) -> i32 {
        self.day
    }

    pub fn get_month(&self) -> i32 {
        self.month
    }

    pub fn get_year(&self) -> i32 {
        self.year
    }

    /// `month` is 1..=12
    pub fn get_month_name(month: i32) -> &'static str {
        debug_assert!(month > 0 && month < 13);
        MONTH_NAMES[(month - 1) as usize]
    }

    /// hh:mm, dd-mm-yyyy
    pub fn get_short_string(&self) -> String {
        format!(
            "{:02}:{:02}, {}-{}-{}",
            self.hour, self.minute, self.day, self.month, self.year
        )
    }

    /// hh:mm.ss, dd month yyyy
    pub fn get_long_string(&self) -> String {
        format!(
            "{:02}:{:02}.{:02}, {} {} {}",
            self.hour,
            self.minute,
            self.second,
            self.day,
            Date::get_month_name(self.month),
            self.year
        )
    }

    /// `Date::Update`: advances the world clock (`world.date`, the only activated date) according
    /// to the game speed, jumping to the next scheduled event when it falls inside the step.
    // PORT: touches game/Ecl (game speed, scheduler, EclGetAccurateTime), so it cannot be `&mut self`
    // on the date that lives inside the world; it always operates on `g.world_mut().date`.
    pub fn update(g: &mut Ctx) {
        if g.world().date.updateme {
            // Update the time

            let nummilliseconds = g.ecl.get_accurate_time() - g.world().date.previousupdate;

            if g.game.game_speed() == GAMESPEED_NORMAL {
                if nummilliseconds > 1000 {
                    g.world_mut().date.advance_second(1);
                    let now = g.ecl.get_accurate_time();
                    g.world_mut().date.previousupdate = now;
                }
            } else if nummilliseconds > 150 {
                let mut newdate = Date::default();
                newdate.set_date_from(&g.world().date);

                match g.game.game_speed() {
                    GAMESPEED_FAST => {
                        newdate.advance_minute(5);
                        newdate.advance_second(3);
                    }
                    GAMESPEED_MEGAFAST => {
                        newdate.advance_hour(1);
                        newdate.advance_minute(5);
                        newdate.advance_second(3);
                    }
                    GAMESPEED_OHMYGOD => {
                        newdate.advance_hour(24);
                        newdate.advance_minute(3);
                        newdate.advance_second(3);
                    }
                    _ => panic!("Unrecognised Game Speed"),
                }

                // C++ GetDateOfNextEvent() hands out a pointer to the event's own rundate,
                // so the advance below changes the scheduled event's date too.
                let next_is_due = matches!(
                    g.world().scheduler.events.first(),
                    Some(ev) if ev.rundate().before(&newdate)
                );

                if next_is_due {
                    let nextevent = &mut g
                        .world_mut()
                        .scheduler
                        .events
                        .first_mut()
                        .expect("Date::Update: no next event")
                        .base_mut()
                        .rundate;
                    nextevent.advance_second(1);
                    let (s, mi, h, d, mo, y) = (
                        nextevent.get_second() + 1,
                        nextevent.get_minute(),
                        nextevent.get_hour(),
                        nextevent.get_day(),
                        nextevent.get_month(),
                        nextevent.get_year(),
                    );
                    g.world_mut().date.set_date(s, mi, h, d, mo, y);
                } else {
                    g.world_mut().date.set_date_from(&newdate);
                }

                let now = g.ecl.get_accurate_time();
                g.world_mut().date.previousupdate = now;
            }
        }

        // Verify Date is valid
        let d = &g.world().date;
        debug_assert!(d.second >= 0 && d.second < 60);
        debug_assert!(d.minute >= 0 && d.minute < 60);
        debug_assert!(d.hour >= 0 && d.hour < 24);

        debug_assert!(d.day > 0 && d.day <= 31);
        debug_assert!(d.month > 0 && d.month <= 12);
        debug_assert!(d.year > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(second: i32, minute: i32, hour: i32, day: i32, month: i32, year: i32) -> Date {
        Date::new(second, minute, hour, day, month, year)
    }

    fn fields(date: &Date) -> (i32, i32, i32, i32, i32, i32) {
        (date.get_second(), date.get_minute(), date.get_hour(), date.get_day(), date.get_month(), date.get_year())
    }

    #[test]
    fn default_date_is_the_cpp_default() {
        assert_eq!(fields(&Date::default()), (1, 1, 1, 1, 1, 1000));
        assert!(!Date::default().updateme);
    }

    #[test]
    fn set_date_rolls_over_dodgy_values() {
        // 75 seconds is a minute and 15 seconds, and the carry runs all the way up
        assert_eq!(fields(&d(75, 5, 5, 5, 5, 2000)), (15, 6, 5, 5, 5, 2000));
        assert_eq!(fields(&d(75, 59, 23, 30, 12, 2000)), (15, 0, 0, 1, 1, 2001));
        // like the C++ (SetDate only calls AdvanceSecond(0)), a minute above 59 with seconds in range is kept
        assert_eq!(fields(&d(0, 61, 5, 5, 5, 2000)), (0, 61, 5, 5, 5, 2000));
    }

    #[test]
    fn advance_rolls_up_through_every_unit() {
        let mut date = d(59, 59, 23, 30, 12, 2000);
        date.advance_second(1);
        assert_eq!(fields(&date), (0, 0, 0, 1, 1, 2001));

        // Months are 30 days long
        let mut date = d(0, 0, 0, 29, 1, 2000);
        date.advance_day(2);
        assert_eq!(fields(&date), (0, 0, 0, 1, 2, 2000));

        let mut date = d(0, 0, 0, 1, 12, 2000);
        date.advance_month(1);
        assert_eq!(fields(&date), (0, 0, 0, 1, 1, 2001));

        let mut date = d(0, 0, 0, 1, 1, 2000);
        date.advance_hour(24);
        assert_eq!(fields(&date), (0, 0, 0, 2, 1, 2000));
        date.advance_minute(90);
        assert_eq!(fields(&date), (0, 30, 1, 2, 1, 2000));
        date.advance_year(3);
        assert_eq!(date.get_year(), 2003);
    }

    #[test]
    fn advance_can_subtract_time() {
        let mut date = d(0, 0, 0, 1, 1, 2000);
        date.advance_second(-1);
        assert_eq!(fields(&date), (59, 59, 23, 30, 12, 1999));

        let mut date = d(0, 0, 12, 15, 6, 2000);
        date.advance_hour(-12);
        assert_eq!(fields(&date), (0, 0, 0, 15, 6, 2000));
        date.advance_hour(-1);
        assert_eq!(fields(&date), (0, 0, 23, 14, 6, 2000));
        date.advance_month(-6);
        assert_eq!(fields(&date), (0, 0, 23, 14, 12, 1999));
    }

    #[test]
    fn advancing_then_going_back_is_the_identity() {
        let start = d(12, 34, 5, 17, 8, 2010);
        for n in [1, 59, 60, 61, 3600, 86_399, 86_400, 1_000_000] {
            let mut date = start.clone();
            date.advance_second(n);
            date.advance_second(-n);
            assert!(date.equal(&start), "n = {n}: {}", date.get_long_string());
        }
    }

    #[test]
    fn comparisons_look_at_the_largest_unit_first() {
        let a = d(59, 59, 23, 30, 12, 2000);
        let b = d(0, 0, 0, 1, 1, 2001);
        assert!(a.before(&b));
        assert!(b.after(&a));
        assert!(!a.after(&b));
        assert!(!a.equal(&b));
        assert!(a.equal(&a.clone()));
        assert!(!a.before(&a.clone()));
        assert!(!a.after(&a.clone()));
        assert!(d(1, 0, 0, 1, 1, 2000).after(&d(0, 0, 0, 1, 1, 2000)));
        assert!(d(0, 0, 0, 1, 2, 2000).after(&d(0, 0, 0, 30, 1, 2000)));
    }

    #[test]
    fn set_date_from_copies_the_time_but_not_the_update_flag() {
        let mut a = Date::default();
        a.activate();
        let b = d(5, 4, 3, 2, 1, 2000);
        a.set_date_from(&b);
        assert!(a.equal(&b));
        assert!(a.updateme);
        a.deactivate();
        assert!(!a.updateme);
    }

    #[test]
    fn date_strings_match_the_cpp_printf_formats() {
        let date = d(5, 4, 3, 2, 1, 2000);
        assert_eq!(date.get_short_string(), "03:04, 2-1-2000");
        assert_eq!(date.get_long_string(), "03:04.05, 2 January 2000");
        assert_eq!(d(0, 30, 23, 30, 12, 1999).get_long_string(), "23:30.00, 30 December 1999");
        assert_eq!(Date::get_month_name(1), "January");
        assert_eq!(Date::get_month_name(12), "December");
    }

    #[test]
    fn date_serde_round_trip_skips_previous_update() {
        let mut date = d(5, 4, 3, 2, 1, 2000);
        date.previousupdate = 1234;
        date.activate();
        let json = serde_json::to_string(&date).unwrap();
        let back: Date = serde_json::from_str(&json).unwrap();
        assert!(back.equal(&date));
        assert!(back.updateme);
        assert_eq!(back.previousupdate, 0);
    }
}
