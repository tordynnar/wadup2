"""pytz stub for WASI.

Provides minimal timezone support for pandas.
"""

import datetime

__version__ = "2024.1"
ZERO = datetime.timedelta(0)


class BaseTzInfo(datetime.tzinfo):
    """Base class for timezone implementations."""
    zone = None

    def __repr__(self):
        return f"<{self.__class__.__name__} {self.zone!r}>"


class _UTC(BaseTzInfo):
    """UTC timezone implementation."""
    zone = "UTC"

    def utcoffset(self, dt):
        return ZERO

    def tzname(self, dt):
        return "UTC"

    def dst(self, dt):
        return ZERO

    def localize(self, dt, is_dst=False):
        if dt.tzinfo is not None:
            raise ValueError("Not naive datetime (tzinfo is already set)")
        return dt.replace(tzinfo=self)

    def normalize(self, dt):
        return dt


UTC = _UTC()
utc = UTC


class _FixedOffset(BaseTzInfo):
    """Fixed offset timezone."""

    def __init__(self, offset, name=None):
        if isinstance(offset, datetime.timedelta):
            self._offset = offset
            total_seconds = int(offset.total_seconds())
        else:
            self._offset = datetime.timedelta(minutes=offset)
            total_seconds = offset * 60

        if name is None:
            hours, remainder = divmod(abs(total_seconds), 3600)
            minutes = remainder // 60
            sign = '-' if total_seconds < 0 else '+'
            name = f"UTC{sign}{hours:02d}:{minutes:02d}"

        self.zone = name
        self._name = name

    def utcoffset(self, dt):
        return self._offset

    def tzname(self, dt):
        return self._name

    def dst(self, dt):
        return ZERO

    def localize(self, dt, is_dst=False):
        if dt.tzinfo is not None:
            raise ValueError("Not naive datetime")
        return dt.replace(tzinfo=self)

    def normalize(self, dt):
        return dt


class UnknownTimeZoneError(KeyError):
    """Raised when a timezone cannot be found."""
    pass


class AmbiguousTimeError(Exception):
    """Raised when a time is ambiguous due to DST."""
    pass


class NonExistentTimeError(Exception):
    """Raised when a time does not exist due to DST."""
    pass


# Common timezone names (simplified - no DST support)
_TIMEZONE_OFFSETS = {
    "UTC": 0,
    "GMT": 0,
    "US/Eastern": -300,
    "US/Central": -360,
    "US/Mountain": -420,
    "US/Pacific": -480,
    "America/New_York": -300,
    "America/Chicago": -360,
    "America/Denver": -420,
    "America/Los_Angeles": -480,
    "Europe/London": 0,
    "Europe/Paris": 60,
    "Europe/Berlin": 60,
    "Asia/Tokyo": 540,
    "Asia/Shanghai": 480,
    "Australia/Sydney": 600,
}

_timezone_cache = {"UTC": UTC}


def timezone(zone):
    """Return a timezone object for the given zone name."""
    if zone in _timezone_cache:
        return _timezone_cache[zone]

    if zone in _TIMEZONE_OFFSETS:
        tz = _FixedOffset(_TIMEZONE_OFFSETS[zone], zone)
        _timezone_cache[zone] = tz
        return tz

    raise UnknownTimeZoneError(zone)


def FixedOffset(offset, name=None):
    """Return a fixed-offset timezone."""
    return _FixedOffset(offset, name)


# Common timezone objects
all_timezones = list(_TIMEZONE_OFFSETS.keys())
all_timezones_set = set(all_timezones)
common_timezones = all_timezones
common_timezones_set = all_timezones_set


def country_timezones(country_code):
    """Return timezone names for a country (stub)."""
    return []


def country_names():
    """Return country codes to names mapping (stub)."""
    return {}
