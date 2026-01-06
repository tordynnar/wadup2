"""dateutil.parser stub for WASI.

Provides date parsing for pandas.
"""

import datetime
import re

from dateutil.tz import tzutc


class parserinfo:
    """Parser configuration."""

    JUMP = [" ", ".", ",", ";", "-", "/", "'",
            "at", "on", "and", "ad", "m", "t", "of",
            "st", "nd", "rd", "th"]

    WEEKDAYS = [("Mon", "Monday"),
                ("Tue", "Tuesday"),
                ("Wed", "Wednesday"),
                ("Thu", "Thursday"),
                ("Fri", "Friday"),
                ("Sat", "Saturday"),
                ("Sun", "Sunday")]

    MONTHS = [("Jan", "January"),
              ("Feb", "February"),
              ("Mar", "March"),
              ("Apr", "April"),
              ("May", "May"),
              ("Jun", "June"),
              ("Jul", "July"),
              ("Aug", "August"),
              ("Sep", "September"),
              ("Oct", "October"),
              ("Nov", "November"),
              ("Dec", "December")]

    HMS = [("h", "hour", "hours"),
           ("m", "minute", "minutes"),
           ("s", "second", "seconds")]

    AMPM = [("am", "a"),
            ("pm", "p")]

    UTCZONE = ["UTC", "GMT", "Z"]

    PERTAIN = ["of"]

    def __init__(self, dayfirst=False, yearfirst=False):
        self.dayfirst = dayfirst
        self.yearfirst = yearfirst


class ParserError(ValueError):
    """Error parsing date string."""
    pass


def _parse_isoformat(timestr):
    """Try to parse ISO format dates."""
    # Try datetime.fromisoformat first (Python 3.7+)
    try:
        return datetime.datetime.fromisoformat(timestr.replace('Z', '+00:00'))
    except ValueError:
        pass

    # Try date only
    try:
        return datetime.datetime.combine(
            datetime.date.fromisoformat(timestr),
            datetime.time()
        )
    except ValueError:
        pass

    return None


def _parse_common_formats(timestr):
    """Try common date formats."""
    formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S.%f",
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d/%m/%Y",
        "%m/%d/%Y",
        "%Y%m%d",
        "%d-%b-%Y",
        "%d %b %Y",
        "%b %d, %Y",
        "%B %d, %Y",
    ]

    for fmt in formats:
        try:
            return datetime.datetime.strptime(timestr.strip(), fmt)
        except ValueError:
            continue

    return None


def parse(timestr, parserinfo=None, **kwargs):
    """Parse a date string into a datetime object.

    This is a simplified implementation that handles common formats.
    """
    if not isinstance(timestr, str):
        raise TypeError("Parser must be given a string")

    timestr = timestr.strip()

    if not timestr:
        raise ParserError("String is empty")

    # Try ISO format first
    result = _parse_isoformat(timestr)
    if result is not None:
        return result

    # Try common formats
    result = _parse_common_formats(timestr)
    if result is not None:
        return result

    raise ParserError(f"Unknown string format: {timestr}")


def isoparse(timestr):
    """Parse an ISO 8601 date string."""
    try:
        return datetime.datetime.fromisoformat(timestr.replace('Z', '+00:00'))
    except ValueError as e:
        raise ParserError(str(e))


# Default parser
_default_parser = None


def _get_default_parser():
    global _default_parser
    if _default_parser is None:
        _default_parser = parser()
    return _default_parser


class parser:
    """Date string parser."""

    def __init__(self, info=None):
        self.info = info or parserinfo()

    def parse(self, timestr, **kwargs):
        return parse(timestr, parserinfo=self.info, **kwargs)
