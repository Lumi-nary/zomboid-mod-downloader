"""
Main window for Zomboid Mod Downloader application.
"""
from PySide6.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QSplitter,
    QListWidget, QListWidgetItem, QPushButton, QLabel, QMenuBar,
    QMenu, QMessageBox, QFileDialog, QDialog, QFormLayout, QLineEdit,
    QCheckBox, QDialogButtonBox, QTabWidget
)
from PySide6.QtCore import Qt, Slot
from PySide6.QtGui import QAction

from ui.browser_widget import WorkshopBrowserWidget
from ui.mods_browser import ModsBrowser
from ui.progress_dialog import ProgressDialog
from core.steamcmd import SteamCMDWrapper
from core.database import ModDatabase
from core.settings import Settings


class MainWindow(QMainWindow):
    """Main application window."""

    def __init__(self):
        """Initialize main window."""
        super().__init__()
        self.setWindowTitle("Zomboid Mod Downloader")

        # Initialize core components
        self.settings = Settings()
        self.database = ModDatabase()
        self.steamcmd: SteamCMDWrapper = None

        # Setup UI
        width, height = self.settings.get_window_size()
        self.resize(width, height)
        self.setup_ui()
        self.setup_menu()

        # Check initial setup
        self._check_initial_setup()

    def setup_ui(self):
        """Setup the user interface."""
        # Central widget
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        # Main layout
        main_layout = QVBoxLayout()
        central_widget.setLayout(main_layout)

        # Create tab widget
        self.tab_widget = QTabWidget()

        # Tab 1: Workshop Browser
        workshop_tab = self._create_workshop_tab()
        self.tab_widget.addTab(workshop_tab, "Workshop Browser")

        # Tab 2: Local Mods
        self.mods_browser = ModsBrowser(self.settings.mod_download_path, self.database)
        self.mods_browser.mods_changed.connect(self._on_mods_changed)
        self.tab_widget.addTab(self.mods_browser, "Local Mods")

        main_layout.addWidget(self.tab_widget)

    def _create_workshop_tab(self) -> QWidget:
        """
        Create the workshop browser tab.

        Returns:
            Workshop tab widget
        """
        tab = QWidget()
        layout = QVBoxLayout()
        tab.setLayout(layout)

        # Create splitter for browser and queue
        splitter = QSplitter(Qt.Horizontal)

        # Left side: Workshop browser
        self.browser = WorkshopBrowserWidget()
        self.browser.mod_added.connect(self._on_mod_added)
        splitter.addWidget(self.browser)

        # Right side: Download queue
        queue_widget = self._create_queue_widget()
        splitter.addWidget(queue_widget)

        # Set splitter sizes (browser 70%, queue 30%)
        splitter.setSizes([700, 300])

        layout.addWidget(splitter)
        return tab

    def _create_queue_widget(self) -> QWidget:
        """
        Create the download queue widget.

        Returns:
            Queue widget
        """
        widget = QWidget()
        layout = QVBoxLayout()

        # Header
        header_label = QLabel("Download Queue")
        header_label.setStyleSheet("font-size: 14pt; font-weight: bold;")
        layout.addWidget(header_label)

        # Queue list
        self.queue_list = QListWidget()
        layout.addWidget(self.queue_list)

        # Queue info label
        self.queue_info_label = QLabel("0 mods in queue")
        layout.addWidget(self.queue_info_label)

        # Buttons
        button_layout = QVBoxLayout()

        self.download_button = QPushButton("Download Mods")
        self.download_button.setStyleSheet("""
            QPushButton {
                background-color: #4CAF50;
                color: white;
                padding: 12px;
                font-size: 12pt;
            }
            QPushButton:hover {
                background-color: #45a049;
            }
        """)
        self.download_button.clicked.connect(self._start_download)
        button_layout.addWidget(self.download_button)

        self.clear_queue_button = QPushButton("Clear Queue")
        self.clear_queue_button.clicked.connect(self._clear_queue)
        button_layout.addWidget(self.clear_queue_button)

        self.remove_selected_button = QPushButton("Remove Selected")
        self.remove_selected_button.clicked.connect(self._remove_selected)
        button_layout.addWidget(self.remove_selected_button)

        layout.addLayout(button_layout)

        widget.setLayout(layout)
        return widget

    def setup_menu(self):
        """Setup the menu bar."""
        menubar = self.menuBar()

        # File menu
        file_menu = menubar.addMenu("File")

        settings_action = QAction("Settings", self)
        settings_action.triggered.connect(self._show_settings)
        file_menu.addAction(settings_action)

        file_menu.addSeparator()

        exit_action = QAction("Exit", self)
        exit_action.triggered.connect(self.close)
        file_menu.addAction(exit_action)

        # View menu
        view_menu = menubar.addMenu("View")

        reload_action = QAction("Reload Browser", self)
        reload_action.triggered.connect(self.browser.reload)
        view_menu.addAction(reload_action)

        home_action = QAction("Go to Workshop Home", self)
        home_action.triggered.connect(self.browser.go_home)
        view_menu.addAction(home_action)

        # Help menu
        help_menu = menubar.addMenu("Help")

        about_action = QAction("About", self)
        about_action.triggered.connect(self._show_about)
        help_menu.addAction(about_action)

    @Slot(str, str)
    def _on_mod_added(self, publishedfileid: str, title: str):
        """
        Handle mod added to queue from browser.

        Args:
            publishedfileid: Workshop item ID
            title: Mod title
        """
        # Check if already in queue
        for i in range(self.queue_list.count()):
            item = self.queue_list.item(i)
            if item.data(Qt.UserRole) == publishedfileid:
                QMessageBox.information(self, "Already in Queue", f"'{title}' is already in the download queue.")
                return

        # Add to database
        self.database.add_to_queue(publishedfileid, title)

        # Add to UI list
        item = QListWidgetItem(f"{title}\nID: {publishedfileid}")
        item.setData(Qt.UserRole, publishedfileid)
        self.queue_list.addItem(item)

        self._update_queue_info()

    def _update_queue_info(self):
        """Update queue information label."""
        count = self.queue_list.count()
        self.queue_info_label.setText(f"{count} mod{'s' if count != 1 else ''} in queue")
        self.download_button.setEnabled(count > 0)

    def _clear_queue(self):
        """Clear all items from the queue."""
        reply = QMessageBox.question(
            self,
            "Clear Queue",
            "Are you sure you want to clear the entire download queue?",
            QMessageBox.Yes | QMessageBox.No,
            QMessageBox.No
        )

        if reply == QMessageBox.Yes:
            self.database.clear_queue()
            self.queue_list.clear()
            self._update_queue_info()

    def _remove_selected(self):
        """Remove selected items from the queue."""
        selected_items = self.queue_list.selectedItems()
        if not selected_items:
            return

        for item in selected_items:
            publishedfileid = item.data(Qt.UserRole)
            self.database.remove_from_queue(publishedfileid)
            row = self.queue_list.row(item)
            self.queue_list.takeItem(row)

        self._update_queue_info()

    def _start_download(self):
        """Start downloading mods from the queue."""
        # Check if SteamCMD is configured
        if not self.settings.steamcmd_path or not self.settings.mod_download_path:
            QMessageBox.warning(
                self,
                "Settings Required",
                "Please configure SteamCMD path and mod download location in Settings."
            )
            self._show_settings()
            return

        # Get all mods from queue
        publishedfileids = []
        for i in range(self.queue_list.count()):
            item = self.queue_list.item(i)
            publishedfileids.append(item.data(Qt.UserRole))

        if not publishedfileids:
            return

        # Create SteamCMD wrapper
        self.steamcmd = SteamCMDWrapper(
            self.settings.steamcmd_path,
            self.settings.mod_download_path
        )

        # Create progress dialog
        progress = ProgressDialog(self)

        # Connect signals
        self.steamcmd.output_received.connect(progress.append_output)
        self.steamcmd.download_started.connect(progress.download_started)
        self.steamcmd.download_finished.connect(
            lambda success, msg: self._on_download_finished(progress, success, msg)
        )
        self.steamcmd.mod_processed.connect(self._on_mod_processed)
        progress.cancel_button.clicked.connect(self.steamcmd.cancel_download)

        # Start download
        username = self.settings.steam_username if not self.settings.use_anonymous_login else ""
        self.steamcmd.download_mods(
            publishedfileids,
            username=username,
            use_anonymous=self.settings.use_anonymous_login
        )

        progress.exec()

    @Slot(str, list)
    def _on_mod_processed(self, publishedfileid: str, folder_names: list):
        """
        Handle individual mod processing - save workshop URL for each created folder.

        Args:
            publishedfileid: Workshop item ID
            folder_names: List of folder names created for this mod
        """
        workshop_url = f"https://steamcommunity.com/sharedfiles/filedetails/?id={publishedfileid}"

        # Get the mod title from the queue
        title = None
        for i in range(self.queue_list.count()):
            item = self.queue_list.item(i)
            if item.data(Qt.UserRole) == publishedfileid:
                title = item.text().split("\n")[0]
                break

        if not title:
            title = f"Workshop Item {publishedfileid}"

        # Save workshop URL for each folder created
        for folder_name in folder_names:
            self.database.add_downloaded_mod(folder_name, title, workshop_url=workshop_url)
            print(f"Saved workshop URL for folder: {folder_name} -> {workshop_url}")

    def _on_download_finished(self, progress: ProgressDialog, success: bool, message: str):
        """
        Handle download completion.

        Args:
            progress: Progress dialog
            success: Whether download succeeded
            message: Completion message
        """
        progress.download_finished(success, message)

        if success:
            # Clear queue if auto-clear is enabled
            if self.settings.auto_clear_queue:
                self.database.clear_queue()
                self.queue_list.clear()
                self._update_queue_info()

            # Refresh local mods and update workshop browser
            self._on_mods_changed()

    def _on_mods_changed(self):
        """Handle mods list changes (refresh browsers)."""
        # Refresh local mods browser
        self.mods_browser.refresh_mods()

        # Update workshop browser with installed mod IDs
        installed_ids = self.mods_browser.get_installed_mod_ids()
        self.browser.set_installed_mods(installed_ids)

    def _show_settings(self):
        """Show settings dialog."""
        dialog = SettingsDialog(self.settings, self)
        if dialog.exec() == QDialog.Accepted:
            # Update mods browser path if changed
            self.mods_browser.set_mod_path(self.settings.mod_download_path)
            self._on_mods_changed()

    def _show_about(self):
        """Show about dialog."""
        QMessageBox.about(
            self,
            "About Zomboid Mod Downloader",
            "<h2>Zomboid Mod Downloader</h2>"
            "<p>A tool for browsing and downloading Project Zomboid mods from Steam Workshop.</p>"
            "<p><b>Features:</b></p>"
            "<ul>"
            "<li>Browse Steam Workshop directly</li>"
            "<li>Add mods to download queue</li>"
            "<li>Batch download with SteamCMD</li>"
            "<li>Track downloaded mods</li>"
            "</ul>"
            "<p>Built with Python and PySide6</p>"
        )

    def _check_initial_setup(self):
        """Check if initial setup is needed."""
        if not self.settings.steamcmd_path or not self.settings.mod_download_path:
            reply = QMessageBox.question(
                self,
                "Initial Setup",
                "SteamCMD and mod download paths are not configured.\nWould you like to configure them now?",
                QMessageBox.Yes | QMessageBox.No,
                QMessageBox.Yes
            )
            if reply == QMessageBox.Yes:
                self._show_settings()

        # Load existing queue from database
        self._load_queue_from_database()

        # Initial mods refresh
        self._on_mods_changed()

    def _load_queue_from_database(self):
        """Load queue items from database."""
        queue_items = self.database.get_queue()
        for item_data in queue_items:
            item = QListWidgetItem(f"{item_data['title']}\nID: {item_data['publishedfileid']}")
            item.setData(Qt.UserRole, item_data['publishedfileid'])
            self.queue_list.addItem(item)

        self._update_queue_info()

    def closeEvent(self, event):
        """Handle window close event."""
        # Save window size
        self.settings.set_window_size(self.width(), self.height())

        # Close database
        self.database.close()

        event.accept()


class SettingsDialog(QDialog):
    """Settings dialog for configuring application."""

    def __init__(self, settings: Settings, parent=None):
        """
        Initialize settings dialog.

        Args:
            settings: Settings object
            parent: Parent widget
        """
        super().__init__(parent)
        self.settings = settings
        self.setWindowTitle("Settings")
        self.setModal(True)
        self.resize(500, 300)
        self.setup_ui()

    def setup_ui(self):
        """Setup the user interface."""
        layout = QFormLayout()

        # SteamCMD path
        steamcmd_layout = QHBoxLayout()
        self.steamcmd_path_edit = QLineEdit(self.settings.steamcmd_path)
        steamcmd_layout.addWidget(self.steamcmd_path_edit)

        steamcmd_browse_btn = QPushButton("Browse...")
        steamcmd_browse_btn.clicked.connect(self._browse_steamcmd)
        steamcmd_layout.addWidget(steamcmd_browse_btn)

        layout.addRow("SteamCMD Path:", steamcmd_layout)

        # Mod download path
        mod_path_layout = QHBoxLayout()
        self.mod_path_edit = QLineEdit(self.settings.mod_download_path)
        mod_path_layout.addWidget(self.mod_path_edit)

        mod_path_browse_btn = QPushButton("Browse...")
        mod_path_browse_btn.clicked.connect(self._browse_mod_path)
        mod_path_layout.addWidget(mod_path_browse_btn)

        layout.addRow("Mod Download Path:", mod_path_layout)

        # Steam login settings
        self.anonymous_checkbox = QCheckBox("Use Anonymous Login")
        self.anonymous_checkbox.setChecked(self.settings.use_anonymous_login)
        self.anonymous_checkbox.toggled.connect(self._on_anonymous_toggled)
        layout.addRow("", self.anonymous_checkbox)

        self.username_edit = QLineEdit(self.settings.steam_username)
        self.username_edit.setEnabled(not self.settings.use_anonymous_login)
        layout.addRow("Steam Username:", self.username_edit)

        # Auto-clear queue
        self.auto_clear_checkbox = QCheckBox("Automatically clear queue after download")
        self.auto_clear_checkbox.setChecked(self.settings.auto_clear_queue)
        layout.addRow("", self.auto_clear_checkbox)

        # Buttons
        buttons = QDialogButtonBox(
            QDialogButtonBox.Ok | QDialogButtonBox.Cancel
        )
        buttons.accepted.connect(self._save_settings)
        buttons.rejected.connect(self.reject)
        layout.addRow(buttons)

        self.setLayout(layout)

    def _browse_steamcmd(self):
        """Browse for SteamCMD executable."""
        file_path, _ = QFileDialog.getOpenFileName(
            self,
            "Select SteamCMD Executable",
            "",
            "Executable Files (*.exe);;All Files (*.*)"
        )
        if file_path:
            self.steamcmd_path_edit.setText(file_path)

    def _browse_mod_path(self):
        """Browse for mod download directory."""
        dir_path = QFileDialog.getExistingDirectory(
            self,
            "Select Mod Download Directory"
        )
        if dir_path:
            self.mod_path_edit.setText(dir_path)

    def _on_anonymous_toggled(self, checked: bool):
        """Handle anonymous login checkbox toggle."""
        self.username_edit.setEnabled(not checked)

    def _save_settings(self):
        """Save settings and close dialog."""
        self.settings.steamcmd_path = self.steamcmd_path_edit.text()
        self.settings.mod_download_path = self.mod_path_edit.text()
        self.settings.use_anonymous_login = self.anonymous_checkbox.isChecked()
        self.settings.steam_username = self.username_edit.text()
        self.settings.auto_clear_queue = self.auto_clear_checkbox.isChecked()

        self.accept()
