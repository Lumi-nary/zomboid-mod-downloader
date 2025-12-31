"""
Settings module for application configuration.
"""
import json
from pathlib import Path
from typing import Optional


class Settings:
    """Manages application settings with persistence."""

    DEFAULT_SETTINGS = {
        "steamcmd_path": "",
        "mod_download_path": "",
        "steam_username": "",
        "use_anonymous_login": True,
        "auto_clear_queue": True,
        "window_width": 1200,
        "window_height": 800,
    }

    def __init__(self, settings_file: str = "settings.json"):
        """Initialize settings manager."""
        self.settings_file = Path(settings_file)
        self.settings = self.DEFAULT_SETTINGS.copy()
        self.load()

    def load(self):
        """Load settings from file."""
        if self.settings_file.exists():
            try:
                with open(self.settings_file, 'r') as f:
                    loaded_settings = json.load(f)
                    self.settings.update(loaded_settings)
            except Exception as e:
                print(f"Error loading settings: {e}")
                # Use default settings if load fails

    def save(self):
        """Save settings to file."""
        try:
            with open(self.settings_file, 'w') as f:
                json.dump(self.settings, f, indent=4)
        except Exception as e:
            print(f"Error saving settings: {e}")

    def get(self, key: str, default=None):
        """Get a setting value."""
        return self.settings.get(key, default)

    def set(self, key: str, value):
        """Set a setting value and save."""
        self.settings[key] = value
        self.save()

    @property
    def steamcmd_path(self) -> str:
        """Get SteamCMD executable path."""
        return self.settings.get("steamcmd_path", "")

    @steamcmd_path.setter
    def steamcmd_path(self, value: str):
        """Set SteamCMD executable path."""
        self.set("steamcmd_path", value)

    @property
    def mod_download_path(self) -> str:
        """Get mod download directory path."""
        return self.settings.get("mod_download_path", "")

    @mod_download_path.setter
    def mod_download_path(self, value: str):
        """Set mod download directory path."""
        self.set("mod_download_path", value)

    @property
    def steam_username(self) -> str:
        """Get Steam username."""
        return self.settings.get("steam_username", "")

    @steam_username.setter
    def steam_username(self, value: str):
        """Set Steam username."""
        self.set("steam_username", value)

    @property
    def use_anonymous_login(self) -> bool:
        """Check if anonymous login is enabled."""
        return self.settings.get("use_anonymous_login", True)

    @use_anonymous_login.setter
    def use_anonymous_login(self, value: bool):
        """Set anonymous login preference."""
        self.set("use_anonymous_login", value)

    @property
    def auto_clear_queue(self) -> bool:
        """Check if queue should auto-clear after download."""
        return self.settings.get("auto_clear_queue", True)

    @auto_clear_queue.setter
    def auto_clear_queue(self, value: bool):
        """Set auto-clear queue preference."""
        self.set("auto_clear_queue", value)

    def get_window_size(self) -> tuple:
        """Get saved window size."""
        width = self.settings.get("window_width", 1200)
        height = self.settings.get("window_height", 800)
        return (width, height)

    def set_window_size(self, width: int, height: int):
        """Save window size."""
        self.settings["window_width"] = width
        self.settings["window_height"] = height
        self.save()
