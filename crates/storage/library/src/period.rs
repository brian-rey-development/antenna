use std::cmp::Reverse;
use std::collections::BTreeMap;

use jiff::civil::Date;
use jiff::{Timestamp, Zoned};

use crate::DocumentSummary;

/// A span of time that groups documents in the library. A greater period is newer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Period {
    /// A calendar month, for a date that is not in the current ISO week.
    Month {
        /// The year.
        year: i16,
        /// The month, from 1 to 12.
        month: i8,
    },
    /// The current ISO 8601 week, without today.
    ThisWeek,
    /// The current civil date.
    Today,
}

/// The documents of one period, from the newest `modified` time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// The period of the documents.
    pub period: Period,
    /// The documents of the period.
    pub documents: Vec<DocumentSummary>,
}

pub(crate) fn period_of(modified: Timestamp, now: &Zoned) -> Period {
    let date = modified.to_zoned(now.time_zone().clone()).date();
    let today = now.date();
    if date == today {
        Period::Today
    } else if iso_week(date) == iso_week(today) {
        Period::ThisWeek
    } else {
        Period::Month {
            year: date.year(),
            month: date.month(),
        }
    }
}

fn iso_week(date: Date) -> (i16, i8) {
    let week = date.iso_week_date();
    (week.year(), week.week())
}

pub(crate) fn group_by_period(mut summaries: Vec<DocumentSummary>, now: &Zoned) -> Vec<Group> {
    summaries.sort_by_key(|summary| Reverse((summary.meta.modified, summary.meta.id)));
    let mut periods: BTreeMap<Period, Vec<DocumentSummary>> = BTreeMap::new();
    for summary in summaries {
        let period = period_of(summary.meta.modified, now);
        periods.entry(period).or_default().push(summary);
    }
    periods
        .into_iter()
        .rev()
        .map(|(period, documents)| Group { period, documents })
        .collect()
}

#[cfg(test)]
mod tests {
    use jiff::civil;
    use jiff::tz::{self, TimeZone};

    use super::*;

    fn zoned(year: i16, month: i8, day: i8) -> Zoned {
        civil::date(year, month, day)
            .at(12, 0, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
    }

    fn period(year: i16, month: i8, day: i8) -> Period {
        period_of(zoned(year, month, day).timestamp(), &zoned(2026, 10, 7))
    }

    #[test]
    fn period_is_today_when_same_date() {
        assert_eq!(period(2026, 10, 7), Period::Today);
    }

    #[test]
    fn period_is_this_week_when_same_iso_week() {
        assert_eq!(period(2026, 10, 5), Period::ThisWeek);
        assert_eq!(period(2026, 10, 11), Period::ThisWeek);
    }

    #[test]
    fn period_is_month_when_other_week() {
        let month = Period::Month {
            year: 2026,
            month: 10,
        };

        assert_eq!(period(2026, 10, 4), month);
        assert_eq!(
            period(2026, 9, 30),
            Period::Month {
                year: 2026,
                month: 9
            }
        );
        assert_eq!(
            period(2025, 10, 7),
            Period::Month {
                year: 2025,
                month: 10
            }
        );
    }

    #[test]
    fn period_is_this_week_when_iso_week_crosses_new_year() {
        let now = zoned(2027, 1, 1);

        let period = period_of(zoned(2026, 12, 29).timestamp(), &now);

        assert_eq!(period, Period::ThisWeek);
    }

    #[test]
    fn period_uses_time_zone_of_now() {
        let now = zoned(2026, 10, 7).with_time_zone(tz::offset(9).to_time_zone());
        let late_utc = zoned(2026, 10, 6).timestamp() + jiff::SignedDuration::from_hours(11);

        assert_eq!(period_of(late_utc, &now), Period::Today);
    }

    #[test]
    fn period_orders_from_oldest_to_newest() {
        let december = Period::Month {
            year: 2025,
            month: 12,
        };
        let january = Period::Month {
            year: 2026,
            month: 1,
        };

        assert!(december < january);
        assert!(january < Period::ThisWeek);
        assert!(Period::ThisWeek < Period::Today);
    }
}
