"""dateutil.tz stub for WASI.

Provides timezone support for pandas.
"""

import datetime

ZERO = datetime.timedelta(0)


class tzutc(datetime.tzinfo):
    """UTC timezone."""

    def utcoffset(self, dt):
        return ZERO

    def dst(self, dt):
        return ZERO

    def tzname(self, dt):
        return "UTC"

    def __repr__(self):
        return "tzutc()"

    def __eq__(self, other):
        return isinstance(other, tzutc)

    def __hash__(self):
        return hash("tzutc")


class tzlocal(datetime.tzinfo):
    """Local timezone (stub - returns UTC)."""

    def utcoffset(self, dt):
        return ZERO

    def dst(self, dt):
        return ZERO

    def tzname(self, dt):
        return "UTC"

    def __repr__(self):
        return "tzlocal()"


class tzoffset(datetime.tzinfo):
    """Fixed offset timezone."""

    def __init__(self, name, offset):
        self._name = name
        if isinstance(offset, datetime.timedelta):
            self._offset = offset
        else:
            self._offset = datetime.timedelta(seconds=offset)

    def utcoffset(self, dt):
        return self._offset

    def dst(self, dt):
        return ZERO

    def tzname(self, dt):
        return self._name

    def __repr__(self):
        return f"tzoffset({self._name!r}, {self._offset.total_seconds()})"


class tzfile(datetime.tzinfo):
    """Timezone from file (stub)."""

    def __init__(self, fileobj=None, filename=None):
        self._filename = filename

    def utcoffset(self, dt):
        return ZERO

    def dst(self, dt):
        return ZERO

    def tzname(self, dt):
        return "UTC"


class tzrange(datetime.tzinfo):
    """Timezone with DST range (stub)."""

    def __init__(self, stdabbr, stdoffset=None, dstabbr=None, dstoffset=None,
                 start=None, end=None):
        self._stdabbr = stdabbr
        self._stdoffset = stdoffset or ZERO
        self._dstabbr = dstabbr
        self._dstoffset = dstoffset or ZERO

    def utcoffset(self, dt):
        return self._stdoffset if isinstance(self._stdoffset, datetime.timedelta) else ZERO

    def dst(self, dt):
        return ZERO

    def tzname(self, dt):
        return self._stdabbr


class tzstr(tzrange):
    """Timezone from string (stub)."""

    def __init__(self, s, posix_offset=False):
        super().__init__(s)


# UTC singleton
UTC = tzutc()


def gettz(name=None):
    """Get a timezone by name."""
    if name is None or name.upper() == "UTC":
        return UTC
    # Return a simple offset timezone
    return tzutc()


def datetime_exists(dt, tz=None):
    """Check if datetime exists in timezone."""
    return True


def datetime_ambiguous(dt, tz=None):
    """Check if datetime is ambiguous in timezone."""
    return False


def resolve_imaginary(dt):
    """Resolve imaginary datetime."""
    return dt


def enfold(dt, fold=1):
    """Add fold attribute to datetime."""
    return dt.replace(fold=fold) if hasattr(dt, 'fold') else dt
