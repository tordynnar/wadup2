"""Data models for file analysis.

This module demonstrates multiple files in a project.
"""


class FileStats:
    """Statistics about a file."""

    def __init__(self, total_bytes, line_count, word_count, char_count, human_size):
        self.total_bytes = total_bytes
        self.line_count = line_count
        self.word_count = word_count
        self.char_count = char_count
        self.human_size = human_size


class EncodingInfo:
    """Encoding detection information from chardet."""

    def __init__(self, encoding, confidence, language, encoding_slug):
        self.encoding = encoding or "unknown"
        self.confidence = confidence or 0.0
        self.language = language or ""
        self.encoding_slug = encoding_slug or "unknown"


class FileAnalysis:
    """Complete analysis of a file."""

    def __init__(self, stats, encoding_info):
        self.stats = stats
        self.encoding_info = encoding_info

    def to_dict(self):
        """Convert analysis to dictionary for WADUP metadata."""
        return {
            'total_bytes': self.stats.total_bytes,
            'line_count': self.stats.line_count,
            'word_count': self.stats.word_count,
            'char_count': self.stats.char_count,
            'human_size': self.stats.human_size,
            'encoding': self.encoding_info.encoding,
            'encoding_confidence': self.encoding_info.confidence,
            'encoding_language': self.encoding_info.language,
            'encoding_slug': self.encoding_info.encoding_slug,
        }
