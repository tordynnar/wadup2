"""File analysis logic.

This module demonstrates:
1. Code separation across multiple files
2. Using a pure-Python dependency (chardet)
"""
import chardet
from .models import FileStats, EncodingInfo, FileAnalysis


def analyze_content(data: bytes) -> FileAnalysis:
    """Analyze the content of a file.

    Args:
        data: Raw bytes from the file

    Returns:
        FileAnalysis object with complete statistics
    """
    stats = compute_stats(data)
    encoding_info = detect_encoding(data)
    return FileAnalysis(stats=stats, encoding_info=encoding_info)


def compute_stats(data: bytes) -> FileStats:
    """Compute basic statistics about file content.

    Args:
        data: Raw bytes from the file

    Returns:
        FileStats with counts
    """
    total_bytes = len(data)

    # Count lines (handle both Unix and Windows line endings)
    line_count = data.count(b'\n')
    if data and not data.endswith(b'\n'):
        line_count += 1  # Count last line without newline

    # Try to decode as text for word/char counts
    try:
        text = data.decode('utf-8')
        word_count = len(text.split())
        char_count = len(text)
    except UnicodeDecodeError:
        # Binary file - use byte-based approximations
        word_count = 0
        char_count = total_bytes

    return FileStats(
        total_bytes=total_bytes,
        line_count=line_count,
        word_count=word_count,
        char_count=char_count,
    )


def detect_encoding(data: bytes) -> EncodingInfo:
    """Detect the encoding of file content using chardet.

    Args:
        data: Raw bytes from the file

    Returns:
        EncodingInfo with detected encoding details
    """
    if not data:
        return EncodingInfo(
            encoding="empty",
            confidence=1.0,
            language="",
        )

    # Use chardet to detect encoding
    result = chardet.detect(data)

    return EncodingInfo(
        encoding=result.get('encoding'),
        confidence=result.get('confidence', 0.0),
        language=result.get('language', ''),
    )
