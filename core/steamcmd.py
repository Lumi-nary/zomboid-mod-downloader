"""
SteamCMD wrapper for downloading Workshop mods.
"""
import subprocess
import shutil
from pathlib import Path
from typing import List, Optional, Callable
from PySide6.QtCore import QProcess, QObject, Signal


class SteamCMDWrapper(QObject):
    """Wrapper for SteamCMD to download Project Zomboid mods."""

    # Signals for progress updates
    output_received = Signal(str)  # Emits SteamCMD output
    download_started = Signal()
    download_finished = Signal(bool, str)  # success, message
    download_progress = Signal(str)  # Progress message
    mod_processed = Signal(str, list)  # publishedfileid, list of folder names created

    PROJECT_ZOMBOID_APP_ID = "108600"

    def __init__(self, steamcmd_path: str, mod_download_path: str):
        """
        Initialize SteamCMD wrapper.

        Args:
            steamcmd_path: Path to steamcmd.exe
            mod_download_path: Directory where mods should be downloaded (final location)
        """
        super().__init__()
        self.steamcmd_path = Path(steamcmd_path)
        self.mod_download_path = Path(mod_download_path)
        self.process: Optional[QProcess] = None
        self.is_running = False
        self.current_download_ids: List[str] = []

    def validate_paths(self) -> tuple[bool, str]:
        """
        Validate that SteamCMD path exists.

        Returns:
            Tuple of (is_valid, error_message)
        """
        if not self.steamcmd_path.exists():
            return False, f"SteamCMD not found at: {self.steamcmd_path}"

        if not self.steamcmd_path.is_file():
            return False, f"SteamCMD path is not a file: {self.steamcmd_path}"

        return True, ""

    def download_mods(
        self,
        publishedfileids: List[str],
        username: str = "",
        password: str = "",
        use_anonymous: bool = True
    ):
        """
        Download mods from Steam Workshop using SteamCMD.

        Args:
            publishedfileids: List of Workshop item IDs to download
            username: Steam username (optional if using anonymous)
            password: Steam password (optional if using anonymous)
            use_anonymous: Use anonymous login instead of credentials
        """
        if self.is_running:
            self.download_finished.emit(False, "Download already in progress")
            return

        valid, error = self.validate_paths()
        if not valid:
            self.download_finished.emit(False, error)
            return

        if not publishedfileids:
            self.download_finished.emit(False, "No mods to download")
            return

        # Store the IDs for post-processing
        self.current_download_ids = publishedfileids

        # Create mod download directory if it doesn't exist
        self.mod_download_path.mkdir(parents=True, exist_ok=True)

        # Build SteamCMD command
        cmd_parts = [str(self.steamcmd_path)]

        # Login
        if use_anonymous:
            cmd_parts.extend(["+login", "anonymous"])
        else:
            if not username:
                self.download_finished.emit(False, "Username required for non-anonymous login")
                return
            if password:
                cmd_parts.extend(["+login", username, password])
            else:
                cmd_parts.extend(["+login", username])

        # Force install directory (important for Workshop content)
        workshop_content_path = self.mod_download_path / "steamapps" / "workshop" / "content" / self.PROJECT_ZOMBOID_APP_ID
        cmd_parts.extend(["+force_install_dir", str(self.mod_download_path)])

        # Add download commands for each mod
        for pfid in publishedfileids:
            cmd_parts.extend([
                "+workshop_download_item",
                self.PROJECT_ZOMBOID_APP_ID,
                pfid
            ])

        # Quit SteamCMD when done
        cmd_parts.append("+quit")

        # Create QProcess for async execution
        self.process = QProcess()

        # Merge stdout and stderr for better output capture
        self.process.setProcessChannelMode(QProcess.ProcessChannelMode.MergedChannels)

        self.process.readyReadStandardOutput.connect(self._handle_stdout)
        self.process.readyReadStandardError.connect(self._handle_stderr)
        self.process.finished.connect(self._handle_finished)

        # Start the process
        self.is_running = True
        self.download_started.emit()

        command_str = " ".join(cmd_parts)
        self.output_received.emit(f"Executing: {command_str}\n")

        # Start SteamCMD
        self.process.start(str(self.steamcmd_path), cmd_parts[1:])

        if not self.process.waitForStarted(5000):
            self.is_running = False
            self.download_finished.emit(False, "Failed to start SteamCMD")

    def _handle_stdout(self):
        """Handle standard output from SteamCMD."""
        if self.process:
            data = self.process.readAllStandardOutput()
            if data:
                output = data.data().decode('utf-8', errors='replace')
                if output:  # Only emit if there's actual output
                    self.output_received.emit(output)
                    print(f"[SteamCMD Output] {output.strip()}")  # Debug print

                    # Parse for progress indicators
                    if "Success" in output or "fully installed" in output:
                        self.download_progress.emit("Download successful")
                    elif "Downloading" in output:
                        self.download_progress.emit("Downloading...")
                    elif "Update state" in output:
                        self.download_progress.emit("Updating...")

    def _handle_stderr(self):
        """Handle standard error from SteamCMD."""
        if self.process:
            data = self.process.readAllStandardError()
            if data:
                output = data.data().decode('utf-8', errors='replace')
                if output:  # Only emit if there's actual output
                    self.output_received.emit(f"ERROR: {output}")
                    print(f"[SteamCMD Error] {output.strip()}")  # Debug print

    def _handle_finished(self, exit_code: int, exit_status):
        """Handle process completion."""
        self.is_running = False

        if exit_code == 0:
            # Process downloaded mods (move from workshop folder to final location)
            self.output_received.emit("\n\nProcessing downloaded mods...\n")
            success, message = self._process_downloaded_mods()

            if success:
                self.download_finished.emit(True, "Download completed and mods processed successfully")
            else:
                self.download_finished.emit(False, f"Download completed but processing failed: {message}")
        else:
            self.download_finished.emit(False, f"SteamCMD exited with code {exit_code}")

    def _process_downloaded_mods(self) -> tuple[bool, str]:
        """
        Process downloaded mods by moving them from workshop folder to final location.

        SteamCMD downloads to: <download_path>/steamapps/workshop/content/108600/<id>/mods/
        We need to move contents to: <mod_download_path>/

        Returns:
            Tuple of (success, message)
        """
        try:
            workshop_base = self.mod_download_path / "steamapps" / "workshop" / "content" / self.PROJECT_ZOMBOID_APP_ID

            if not workshop_base.exists():
                return False, f"Workshop folder not found: {workshop_base}"

            processed_count = 0
            for publishedfileid in self.current_download_ids:
                workshop_mod_folder = workshop_base / publishedfileid

                if not workshop_mod_folder.exists():
                    self.output_received.emit(f"⚠ Warning: Mod {publishedfileid} not found in workshop folder\n")
                    continue

                # Track which folders were created for this workshop ID
                created_folders = []

                # Check if there's a 'mods' subfolder (common in PZ workshop items)
                mods_subfolder = workshop_mod_folder / "mods"

                if mods_subfolder.exists() and mods_subfolder.is_dir():
                    # Move contents from the mods subfolder
                    self.output_received.emit(f"Processing mod {publishedfileid}...\n")

                    for item in mods_subfolder.iterdir():
                        dest = self.mod_download_path / item.name

                        # Remove destination if it exists
                        if dest.exists():
                            if dest.is_dir():
                                shutil.rmtree(dest)
                            else:
                                dest.unlink()

                        # Move the mod folder
                        shutil.move(str(item), str(dest))
                        self.output_received.emit(f"  ✓ Moved {item.name} to {self.mod_download_path}\n")
                        created_folders.append(item.name)

                    processed_count += 1
                else:
                    # No mods subfolder, move the entire workshop folder
                    self.output_received.emit(f"Processing mod {publishedfileid} (no mods subfolder)...\n")
                    dest = self.mod_download_path / publishedfileid

                    if dest.exists():
                        if dest.is_dir():
                            shutil.rmtree(dest)
                        else:
                            dest.unlink()

                    shutil.move(str(workshop_mod_folder), str(dest))
                    self.output_received.emit(f"  ✓ Moved {publishedfileid} to {self.mod_download_path}\n")
                    created_folders.append(publishedfileid)
                    processed_count += 1

                # Emit signal with the mapping of workshop ID to created folders
                if created_folders:
                    self.mod_processed.emit(publishedfileid, created_folders)

            # Clean up steamapps folder (entire temporary structure)
            steamapps_folder = self.mod_download_path / "steamapps"
            try:
                if steamapps_folder.exists():
                    shutil.rmtree(steamapps_folder)
                    self.output_received.emit("\nCleaned up temporary SteamCMD files\n")
            except Exception as e:
                self.output_received.emit(f"\n⚠ Warning: Could not clean up steamapps folder: {e}\n")

            self.output_received.emit(f"\n✓ Successfully processed {processed_count} mod(s)\n")
            return True, f"Processed {processed_count} mods"

        except Exception as e:
            error_msg = f"Error processing mods: {e}"
            self.output_received.emit(f"\n✗ {error_msg}\n")
            return False, error_msg

    def cancel_download(self):
        """Cancel the current download."""
        if self.process and self.is_running:
            self.output_received.emit("\nCancelling download...\n")
            self.process.kill()
            self.is_running = False
            self.download_finished.emit(False, "Download cancelled by user")

    def get_workshop_mod_path(self, publishedfileid: str) -> Path:
        """
        Get the expected path for a downloaded Workshop mod.

        Args:
            publishedfileid: Workshop item ID

        Returns:
            Path to the mod directory
        """
        return (
            self.mod_download_path
            / "steamapps"
            / "workshop"
            / "content"
            / self.PROJECT_ZOMBOID_APP_ID
            / publishedfileid
        )
