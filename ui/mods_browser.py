"""
Local mods browser widget for managing installed mods.
"""
from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QTableWidget, QTableWidgetItem,
    QPushButton, QLabel, QMessageBox, QHeaderView, QMenu, QSplitter,
    QListWidget, QListWidgetItem, QTextEdit, QFrame
)
from PySide6.QtCore import Qt, Signal, QUrl
from PySide6.QtGui import QDesktopServices, QAction, QPixmap
from pathlib import Path
import shutil


class ModsBrowser(QWidget):
    """Widget for browsing and managing local mods."""

    # Signal emitted when mods list changes
    mods_changed = Signal()

    def __init__(self, mod_path: str, database=None, parent=None):
        """
        Initialize the mods browser.

        Args:
            mod_path: Path to the mods directory
            database: ModDatabase instance for workshop URLs
            parent: Parent widget
        """
        super().__init__(parent)
        self.mod_path = Path(mod_path) if mod_path else None
        self.database = database
        self.current_mod_folder = None
        self.setup_ui()

    def setup_ui(self):
        """Setup the user interface."""
        layout = QVBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)

        # Create splitter for details and list
        splitter = QSplitter(Qt.Orientation.Horizontal)

        # Left side: Mod details panel
        details_panel = self._create_details_panel()
        splitter.addWidget(details_panel)

        # Right side: Mods list
        list_panel = self._create_list_panel()
        splitter.addWidget(list_panel)

        # Set splitter sizes (details 40%, list 60%)
        splitter.setSizes([400, 600])

        layout.addWidget(splitter)
        self.setLayout(layout)

    def _create_details_panel(self) -> QWidget:
        """
        Create the mod details panel.

        Returns:
            Details panel widget
        """
        panel = QFrame()
        panel.setFrameStyle(QFrame.Shape.StyledPanel)

        layout = QVBoxLayout()

        # Poster/thumbnail image
        self.poster_label = QLabel()
        self.poster_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.poster_label.setMinimumHeight(200)
        self.poster_label.setMaximumHeight(300)
        self.poster_label.setStyleSheet("""
            QLabel {
                border: 2px solid #3c3c3c;
                border-radius: 4px;
            }
        """)
        self.poster_label.setScaledContents(False)
        layout.addWidget(self.poster_label)

        # Mod name label
        self.detail_name_label = QLabel("Select a mod to view details")
        self.detail_name_label.setStyleSheet("""
            font-size: 16pt;
            font-weight: bold;
            padding: 10px;
        """)
        self.detail_name_label.setWordWrap(True)
        layout.addWidget(self.detail_name_label)

        # Mod info text
        self.detail_info_text = QTextEdit()
        self.detail_info_text.setReadOnly(True)
        self.detail_info_text.setStyleSheet("""
            QTextEdit {
                padding: 10px;
            }
        """)
        layout.addWidget(self.detail_info_text)

        # Action buttons
        button_layout = QHBoxLayout()

        self.open_folder_btn = QPushButton("Open Folder")
        self.open_folder_btn.setEnabled(False)
        self.open_folder_btn.clicked.connect(self._open_selected_folder)
        button_layout.addWidget(self.open_folder_btn)

        self.workshop_link_btn = QPushButton("Steam Workshop")
        self.workshop_link_btn.setEnabled(False)
        self.workshop_link_btn.clicked.connect(self._open_selected_workshop)
        button_layout.addWidget(self.workshop_link_btn)

        self.delete_btn = QPushButton("Delete")
        self.delete_btn.setEnabled(False)
        self.delete_btn.clicked.connect(self._delete_selected_mod)
        self.delete_btn.setStyleSheet("""
            QPushButton {
                background-color: #d32f2f;
                padding: 8px 16px;
            }
            QPushButton:hover {
                background-color: #b71c1c;
            }
        """)
        button_layout.addWidget(self.delete_btn)

        layout.addLayout(button_layout)

        panel.setLayout(layout)
        return panel

    def _create_list_panel(self) -> QWidget:
        """
        Create the mods list panel.

        Returns:
            List panel widget
        """
        panel = QWidget()
        layout = QVBoxLayout()

        # Header with count and refresh
        header_layout = QHBoxLayout()

        self.mod_count_label = QLabel("Active [0]")
        self.mod_count_label.setStyleSheet("""
            font-size: 12pt;
            font-weight: bold;
        """)
        header_layout.addWidget(self.mod_count_label)

        header_layout.addStretch()

        self.refresh_button = QPushButton("Refresh")
        self.refresh_button.clicked.connect(self.refresh_mods)
        header_layout.addWidget(self.refresh_button)

        layout.addLayout(header_layout)

        # Mods list
        self.mods_list = QListWidget()
        self.mods_list.itemSelectionChanged.connect(self._on_mod_selected)
        self.mods_list.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.mods_list.customContextMenuRequested.connect(self._show_context_menu)
        layout.addWidget(self.mods_list)

        # Info label
        self.info_label = QLabel("Set mod path in Settings to browse installed mods")
        self.info_label.setStyleSheet("color: #888888; font-style: italic; padding: 5px;")
        layout.addWidget(self.info_label)

        panel.setLayout(layout)
        return panel

    def set_mod_path(self, mod_path: str):
        """
        Update the mod path and refresh the list.

        Args:
            mod_path: New path to the mods directory
        """
        self.mod_path = Path(mod_path) if mod_path else None
        self.refresh_mods()

    def refresh_mods(self):
        """Refresh the mods list."""
        self.mods_list.clear()
        self.current_mod_folder = None
        self._clear_details()

        if not self.mod_path or not self.mod_path.exists():
            self.info_label.setText("Mod path not set or doesn't exist. Set it in Settings.")
            self.mod_count_label.setText("Active [0]")
            return

        # Find all mod folders
        mod_folders = [f for f in self.mod_path.iterdir() if f.is_dir() and not f.name.startswith('.')]

        self.mod_count_label.setText(f"Active [{len(mod_folders)}]")

        if len(mod_folders) == 0:
            self.info_label.setText("No mods found in the mods directory")
            return

        self.info_label.setText("")

        # Populate list
        for folder in sorted(mod_folders, key=lambda x: x.name.lower()):
            self._add_mod_to_list(folder)

    def _add_mod_to_list(self, mod_folder: Path):
        """
        Add a mod to the list.

        Args:
            mod_folder: Path to the mod folder
        """
        # Try to find mod name from mod.info or use folder name
        mod_name = self._get_mod_name(mod_folder)

        # Create list item
        item = QListWidgetItem(mod_name)
        item.setData(Qt.ItemDataRole.UserRole, str(mod_folder))
        self.mods_list.addItem(item)

    def _on_mod_selected(self):
        """Handle mod selection change."""
        selected_items = self.mods_list.selectedItems()
        if not selected_items:
            self._clear_details()
            return

        item = selected_items[0]
        mod_folder_path = item.data(Qt.ItemDataRole.UserRole)
        self.current_mod_folder = Path(mod_folder_path)

        self._update_details(self.current_mod_folder)

    def _clear_details(self):
        """Clear the details panel."""
        self.poster_label.clear()
        self.poster_label.setText("")
        self.detail_name_label.setText("Select a mod to view details")
        self.detail_info_text.clear()
        self.open_folder_btn.setEnabled(False)
        self.workshop_link_btn.setEnabled(False)
        self.delete_btn.setEnabled(False)

    def _update_details(self, mod_folder: Path):
        """
        Update the details panel with mod information.

        Args:
            mod_folder: Path to the mod folder
        """
        # Get mod name
        mod_name = self._get_mod_name(mod_folder)
        self.detail_name_label.setText(mod_name)

        # Load and display poster/thumbnail if it exists
        poster_path = mod_folder / "poster.png"
        if poster_path.exists():
            pixmap = QPixmap(str(poster_path))
            if not pixmap.isNull():
                # Scale to fit while maintaining aspect ratio
                scaled_pixmap = pixmap.scaled(
                    self.poster_label.width() - 10,
                    self.poster_label.maximumHeight() - 10,
                    Qt.AspectRatioMode.KeepAspectRatio,
                    Qt.TransformationMode.SmoothTransformation
                )
                self.poster_label.setPixmap(scaled_pixmap)
            else:
                self.poster_label.clear()
                self.poster_label.setText("No poster available")
        else:
            self.poster_label.clear()
            self.poster_label.setText("No poster available")

        # Build info text in Steam Workshop style
        info_parts = []

        # Read mod.info to extract details
        mod_info_path = mod_folder / "mod.info"
        mod_info_data = {}
        if mod_info_path.exists():
            try:
                with open(mod_info_path, 'r', encoding='utf-8', errors='ignore') as f:
                    for line in f:
                        line = line.strip()
                        if '=' in line and not line.startswith('#'):
                            key, value = line.split('=', 1)
                            key = key.strip()
                            value = value.strip()
                            if key not in mod_info_data:  # Keep first occurrence
                                mod_info_data[key] = value
                            elif key == 'description':  # Concatenate descriptions
                                mod_info_data[key] += '\n' + value
            except Exception:
                pass

        # Display in Steam Workshop style format
        if 'name' in mod_info_data:
            info_parts.append(f"<b>Name:</b><br>{mod_info_data['name']}")

        if 'id' in mod_info_data:
            info_parts.append(f"<b>PackageID:</b><br>{mod_info_data['id']}")

        # Authors
        authors = mod_info_data.get('authors', 'Unknown')
        info_parts.append(f"<b>Authors:</b><br>{authors}")

        # Mod Version
        mod_version = mod_info_data.get('modversion', 'Not specified')
        info_parts.append(f"<b>Mod Version:</b><br>{mod_version}")

        # Supported Version
        pz_version = mod_info_data.get('pzversion', 'Unknown')
        info_parts.append(f"<b>Supported Version:</b><br>{pz_version}")

        # Folder size
        size = self._get_folder_size(mod_folder)
        info_parts.append(f"<b>Folder Size:</b><br>{self._format_size(size)}")

        # Path
        info_parts.append(f"<b>Path:</b><br>{mod_folder}")

        # Last modified
        try:
            mtime = mod_folder.stat().st_mtime
            from datetime import datetime
            mod_time = datetime.fromtimestamp(mtime).strftime('%Y-%m-%d %H:%M:%S')
            info_parts.append(f"<b>Last Touched:</b><br>{mod_time}")
        except Exception:
            pass

        # Workshop times (if available from database)
        if self.database and mod_folder.name:
            workshop_url = self.database.get_mod_workshop_url(mod_folder.name)
            if workshop_url:
                info_parts.append(f"<b>Workshop Times:</b><br>Downloaded from Steam Workshop")

        # Join with line breaks
        self.detail_info_text.setHtml('<br><br>'.join(info_parts))

        # Enable buttons
        self.open_folder_btn.setEnabled(True)
        self.delete_btn.setEnabled(True)

        # Enable workshop link if we have a database-stored URL or if folder name is numeric
        workshop_url = None
        if self.database:
            # Try to get URL from database (works for any folder name if we have it stored)
            workshop_url = self.database.get_mod_workshop_url(mod_folder.name)

        if workshop_url:
            # We have a stored workshop URL
            self.workshop_link_btn.setEnabled(True)
        elif mod_folder.name.isdigit():
            # Folder name is a workshop ID, can construct URL
            self.workshop_link_btn.setEnabled(True)
        else:
            # No workshop URL available
            self.workshop_link_btn.setEnabled(False)

    def _get_mod_name(self, mod_folder: Path) -> str:
        """
        Get the mod name from mod.info file or use folder name.

        Args:
            mod_folder: Path to the mod folder

        Returns:
            Mod name
        """
        mod_info = mod_folder / "mod.info"
        if mod_info.exists():
            try:
                with open(mod_info, 'r', encoding='utf-8', errors='ignore') as f:
                    for line in f:
                        if line.strip().startswith('name='):
                            name = line.split('=', 1)[1].strip()
                            return name
            except Exception:
                pass

        return mod_folder.name

    def _get_folder_size(self, folder: Path) -> int:
        """
        Get the total size of a folder.

        Args:
            folder: Path to the folder

        Returns:
            Size in bytes
        """
        total = 0
        try:
            for item in folder.rglob('*'):
                if item.is_file():
                    total += item.stat().st_size
        except Exception:
            pass
        return total

    def _format_size(self, size: int) -> str:
        """
        Format size in bytes to human-readable format.

        Args:
            size: Size in bytes

        Returns:
            Formatted size string
        """
        for unit in ['B', 'KB', 'MB', 'GB']:
            if size < 1024.0:
                return f"{size:.1f} {unit}"
            size /= 1024.0
        return f"{size:.1f} TB"

    def _open_selected_folder(self):
        """Open the currently selected mod folder in file explorer."""
        if self.current_mod_folder:
            QDesktopServices.openUrl(QUrl.fromLocalFile(str(self.current_mod_folder)))

    def _open_selected_workshop(self):
        """Open the Steam Workshop page for the selected mod."""
        if not self.current_mod_folder:
            return

        # Try to get URL from database first (works for any folder name)
        url = None
        if self.database:
            url = self.database.get_mod_workshop_url(self.current_mod_folder.name)

        # Fallback to constructed URL if folder name is numeric
        if not url and self.current_mod_folder.name.isdigit():
            url = f"https://steamcommunity.com/sharedfiles/filedetails/?id={self.current_mod_folder.name}"

        # Open URL if we have one
        if url:
            QDesktopServices.openUrl(QUrl(url))
        else:
            # This shouldn't happen if the button is properly disabled
            from PySide6.QtWidgets import QMessageBox
            QMessageBox.warning(
                self,
                "No Workshop URL",
                f"No Steam Workshop URL available for '{self.current_mod_folder.name}'.\n\n"
                "This mod may not have been downloaded through the Workshop Browser."
            )

    def _delete_selected_mod(self):
        """Delete the currently selected mod after confirmation."""
        if not self.current_mod_folder:
            return

        reply = QMessageBox.question(
            self,
            "Delete Mod",
            f"Are you sure you want to delete '{self.current_mod_folder.name}'?\n\nThis action cannot be undone.",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No
        )

        if reply == QMessageBox.StandardButton.Yes:
            try:
                mod_name = self.current_mod_folder.name
                shutil.rmtree(self.current_mod_folder)
                self.refresh_mods()
                self.mods_changed.emit()
                QMessageBox.information(self, "Success", f"Mod '{mod_name}' deleted successfully.")
            except Exception as e:
                QMessageBox.critical(self, "Error", f"Failed to delete mod:\n{e}")

    def _show_context_menu(self, position):
        """
        Show context menu for mod items.

        Args:
            position: Menu position
        """
        item = self.mods_list.itemAt(position)
        if item is None:
            return

        mod_folder_path = item.data(Qt.ItemDataRole.UserRole)
        mod_folder = Path(mod_folder_path)

        menu = QMenu(self)

        open_action = QAction("Open Folder", self)
        open_action.triggered.connect(lambda: QDesktopServices.openUrl(QUrl.fromLocalFile(str(mod_folder))))
        menu.addAction(open_action)

        workshop_action = QAction("View on Steam Workshop", self)
        if mod_folder.name.isdigit():
            workshop_action.triggered.connect(lambda: QDesktopServices.openUrl(
                QUrl(f"https://steamcommunity.com/sharedfiles/filedetails/?id={mod_folder.name}")
            ))
        else:
            workshop_action.setEnabled(False)
        menu.addAction(workshop_action)

        menu.addSeparator()

        delete_action = QAction("Delete Mod", self)
        delete_action.triggered.connect(lambda: self._delete_mod_by_path(mod_folder))
        menu.addAction(delete_action)

        menu.exec(self.mods_list.viewport().mapToGlobal(position))

    def _delete_mod_by_path(self, mod_folder: Path):
        """
        Delete a mod by its path (used by context menu).

        Args:
            mod_folder: Path to the mod folder
        """
        reply = QMessageBox.question(
            self,
            "Delete Mod",
            f"Are you sure you want to delete '{mod_folder.name}'?\n\nThis action cannot be undone.",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No
        )

        if reply == QMessageBox.StandardButton.Yes:
            try:
                mod_name = mod_folder.name
                shutil.rmtree(mod_folder)
                self.refresh_mods()
                self.mods_changed.emit()
                QMessageBox.information(self, "Success", f"Mod '{mod_name}' deleted successfully.")
            except Exception as e:
                QMessageBox.critical(self, "Error", f"Failed to delete mod:\n{e}")

    def get_installed_mod_ids(self) -> set:
        """
        Get a set of installed mod IDs (workshop IDs).

        This checks both:
        1. Folders with numeric names (workshop IDs)
        2. Database records that map folder names to workshop URLs

        Returns:
            Set of Workshop IDs
        """
        if not self.mod_path or not self.mod_path.exists():
            return set()

        mod_ids = set()

        # Get all installed mod folders
        installed_folders = [f for f in self.mod_path.iterdir() if f.is_dir() and not f.name.startswith('.')]

        for folder in installed_folders:
            # If folder name is numeric, it's a workshop ID
            if folder.name.isdigit():
                mod_ids.add(folder.name)
            # Otherwise, check database for workshop URL
            elif self.database:
                workshop_url = self.database.get_mod_workshop_url(folder.name)
                if workshop_url:
                    # Extract workshop ID from URL
                    # URL format: https://steamcommunity.com/sharedfiles/filedetails/?id=12345678
                    import re
                    match = re.search(r'[?&]id=(\d+)', workshop_url)
                    if match:
                        mod_ids.add(match.group(1))

        return mod_ids
