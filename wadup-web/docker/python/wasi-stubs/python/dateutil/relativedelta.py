"""dateutil.relativedelta stub for WASI.

Provides relative date calculations for pandas.
"""

import datetime
import calendar


class relativedelta:
    """Relative delta for date arithmetic."""

    def __init__(self, dt1=None, dt2=None,
                 years=0, months=0, days=0, leapdays=0,
                 weeks=0, hours=0, minutes=0, seconds=0, microseconds=0,
                 year=None, month=None, day=None, weekday=None,
                 yearday=None, nlyearday=None,
                 hour=None, minute=None, second=None, microsecond=None):

        if dt1 and dt2:
            # Calculate difference between two dates
            if not isinstance(dt1, datetime.date):
                dt1 = datetime.datetime.combine(dt1, datetime.time())
            if not isinstance(dt2, datetime.date):
                dt2 = datetime.datetime.combine(dt2, datetime.time())

            self.years = 0
            self.months = 0
            self.days = (dt1 - dt2).days
            self.hours = 0
            self.minutes = 0
            self.seconds = 0
            self.microseconds = 0
        else:
            self.years = years
            self.months = months
            self.days = days + weeks * 7
            self.hours = hours
            self.minutes = minutes
            self.seconds = seconds
            self.microseconds = microseconds

        self.leapdays = leapdays
        self.year = year
        self.month = month
        self.day = day
        self.weekday = weekday
        self.hour = hour
        self.minute = minute
        self.second = second
        self.microsecond = microsecond
        self.yearday = yearday
        self.nlyearday = nlyearday

    def __add__(self, other):
        if isinstance(other, relativedelta):
            return relativedelta(
                years=self.years + other.years,
                months=self.months + other.months,
                days=self.days + other.days,
                hours=self.hours + other.hours,
                minutes=self.minutes + other.minutes,
                seconds=self.seconds + other.seconds,
                microseconds=self.microseconds + other.microseconds,
            )

        if isinstance(other, datetime.timedelta):
            return relativedelta(
                years=self.years,
                months=self.months,
                days=self.days + other.days,
                hours=self.hours,
                minutes=self.minutes,
                seconds=self.seconds + other.seconds,
                microseconds=self.microseconds + other.microseconds,
            )

        if isinstance(other, (datetime.datetime, datetime.date)):
            return self._add_to_date(other)

        return NotImplemented

    def __radd__(self, other):
        return self.__add__(other)

    def __sub__(self, other):
        if isinstance(other, relativedelta):
            return relativedelta(
                years=self.years - other.years,
                months=self.months - other.months,
                days=self.days - other.days,
                hours=self.hours - other.hours,
                minutes=self.minutes - other.minutes,
                seconds=self.seconds - other.seconds,
                microseconds=self.microseconds - other.microseconds,
            )
        return NotImplemented

    def __rsub__(self, other):
        if isinstance(other, (datetime.datetime, datetime.date)):
            return self.__neg__().__add__(other)
        return NotImplemented

    def __neg__(self):
        return relativedelta(
            years=-self.years,
            months=-self.months,
            days=-self.days,
            hours=-self.hours,
            minutes=-self.minutes,
            seconds=-self.seconds,
            microseconds=-self.microseconds,
        )

    def _add_to_date(self, other):
        """Add this relativedelta to a date/datetime."""
        if isinstance(other, datetime.date) and not isinstance(other, datetime.datetime):
            other = datetime.datetime.combine(other, datetime.time())

        year = other.year + self.years
        month = other.month + self.months

        # Normalize month
        while month > 12:
            month -= 12
            year += 1
        while month < 1:
            month += 12
            year -= 1

        # Handle day overflow
        day = min(other.day, calendar.monthrange(year, month)[1])

        if self.year is not None:
            year = self.year
        if self.month is not None:
            month = self.month
        if self.day is not None:
            day = self.day

        result = other.replace(year=year, month=month, day=day)

        # Apply time components
        result += datetime.timedelta(
            days=self.days,
            hours=self.hours,
            minutes=self.minutes,
            seconds=self.seconds,
            microseconds=self.microseconds,
        )

        # Apply absolute time components
        if self.hour is not None:
            result = result.replace(hour=self.hour)
        if self.minute is not None:
            result = result.replace(minute=self.minute)
        if self.second is not None:
            result = result.replace(second=self.second)
        if self.microsecond is not None:
            result = result.replace(microsecond=self.microsecond)

        return result

    def __repr__(self):
        parts = []
        if self.years:
            parts.append(f"years={self.years:+d}")
        if self.months:
            parts.append(f"months={self.months:+d}")
        if self.days:
            parts.append(f"days={self.days:+d}")
        if self.hours:
            parts.append(f"hours={self.hours:+d}")
        if self.minutes:
            parts.append(f"minutes={self.minutes:+d}")
        if self.seconds:
            parts.append(f"seconds={self.seconds:+d}")
        if self.microseconds:
            parts.append(f"microseconds={self.microseconds:+d}")
        return f"relativedelta({', '.join(parts)})"

    def __bool__(self):
        return bool(self.years or self.months or self.days or
                    self.hours or self.minutes or self.seconds or
                    self.microseconds)


# Weekday constants
MO = 0
TU = 1
WE = 2
TH = 3
FR = 4
SA = 5
SU = 6
