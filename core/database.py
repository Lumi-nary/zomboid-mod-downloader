"""
Database module for tracking downloaded mods and application state.
"""
import sqlite3
from pathlib import Path
from typing import List, Optional, Dict
from datetime import datetime


class ModDatabase:
    """Handles database operations for tracking downloaded mods."""

    def __init__(self, db_path: str = "zomboid_mods.db"):
        """Initialize database connection and create tables if needed."""
        self.db_path = Path(db_path)
        self.conn = None
        self._connect()
        self._create_tables()

    def _connect(self):
        """Establish database connection."""
        self.conn = sqlite3.connect(self.db_path)
        self.conn.row_factory = sqlite3.Row  # Enable column access by name

    def _create_tables(self):
        """Create necessary database tables."""
        cursor = self.conn.cursor()

        # Downloaded mods table - check if workshop_url column exists, add if not
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS downloaded_mods (
                publishedfileid TEXT PRIMARY KEY,
                title TEXT,
                download_date TEXT,
                file_size INTEGER,
                last_updated TEXT,
                workshop_url TEXT
            )
        """)

        # Add workshop_url column if it doesn't exist (for existing databases)
        try:
            cursor.execute("ALTER TABLE downloaded_mods ADD COLUMN workshop_url TEXT")
            self.conn.commit()
        except sqlite3.OperationalError:
            pass  # Column already exists

        # Download queue table
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS download_queue (
                publishedfileid TEXT PRIMARY KEY,
                title TEXT,
                added_date TEXT
            )
        """)

        self.conn.commit()

    def add_downloaded_mod(self, publishedfileid: str, title: str, file_size: int = 0, workshop_url: str = ""):
        """Add a mod to the downloaded mods list."""
        cursor = self.conn.cursor()
        now = datetime.now().isoformat()

        cursor.execute("""
            INSERT OR REPLACE INTO downloaded_mods
            (publishedfileid, title, download_date, file_size, last_updated, workshop_url)
            VALUES (?, ?, ?, ?, ?, ?)
        """, (publishedfileid, title, now, file_size, now, workshop_url))

        self.conn.commit()

    def get_mod_workshop_url(self, publishedfileid: str) -> Optional[str]:
        """Get workshop URL for a mod."""
        cursor = self.conn.cursor()
        cursor.execute(
            "SELECT workshop_url FROM downloaded_mods WHERE publishedfileid = ?",
            (publishedfileid,)
        )
        result = cursor.fetchone()
        return result['workshop_url'] if result else None

    def is_mod_downloaded(self, publishedfileid: str) -> bool:
        """Check if a mod has been downloaded."""
        cursor = self.conn.cursor()
        cursor.execute(
            "SELECT 1 FROM downloaded_mods WHERE publishedfileid = ?",
            (publishedfileid,)
        )
        return cursor.fetchone() is not None

    def get_downloaded_mods(self) -> List[Dict]:
        """Get list of all downloaded mods."""
        cursor = self.conn.cursor()
        cursor.execute("SELECT * FROM downloaded_mods ORDER BY download_date DESC")
        return [dict(row) for row in cursor.fetchall()]

    def remove_downloaded_mod(self, publishedfileid: str):
        """Remove a mod from downloaded list."""
        cursor = self.conn.cursor()
        cursor.execute(
            "DELETE FROM downloaded_mods WHERE publishedfileid = ?",
            (publishedfileid,)
        )
        self.conn.commit()

    def add_to_queue(self, publishedfileid: str, title: str):
        """Add a mod to the download queue."""
        cursor = self.conn.cursor()
        now = datetime.now().isoformat()

        cursor.execute("""
            INSERT OR REPLACE INTO download_queue
            (publishedfileid, title, added_date)
            VALUES (?, ?, ?)
        """, (publishedfileid, title, now))

        self.conn.commit()

    def remove_from_queue(self, publishedfileid: str):
        """Remove a mod from the download queue."""
        cursor = self.conn.cursor()
        cursor.execute(
            "DELETE FROM download_queue WHERE publishedfileid = ?",
            (publishedfileid,)
        )
        self.conn.commit()

    def get_queue(self) -> List[Dict]:
        """Get all mods in the download queue."""
        cursor = self.conn.cursor()
        cursor.execute("SELECT * FROM download_queue ORDER BY added_date")
        return [dict(row) for row in cursor.fetchall()]

    def clear_queue(self):
        """Clear all mods from the download queue."""
        cursor = self.conn.cursor()
        cursor.execute("DELETE FROM download_queue")
        self.conn.commit()

    def close(self):
        """Close database connection."""
        if self.conn:
            self.conn.close()
