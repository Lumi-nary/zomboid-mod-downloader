"""
Zomboid Mod Downloader - Main entry point.

A desktop application for browsing and downloading Project Zomboid mods
from Steam Workshop using SteamCMD.
"""
import sys
from PySide6.QtWidgets import QApplication
from ui.main_window import MainWindow


def main():
    """Main entry point for the application."""
    # Create application
    app = QApplication(sys.argv)
    app.setApplicationName("Zomboid Mod Downloader")
    app.setOrganizationName("ZomboidModDownloader")

    # Apply dark mode theme
    app.setStyle("Fusion")

    dark_stylesheet = """
    QWidget {
        background-color: #1e1e1e;
        color: #d4d4d4;
        font-family: 'Segoe UI', Arial, sans-serif;
        font-size: 9pt;
    }

    QMainWindow {
        background-color: #1e1e1e;
    }

    QMenuBar {
        background-color: #2d2d30;
        color: #d4d4d4;
        border-bottom: 1px solid #3e3e42;
    }

    QMenuBar::item {
        background-color: transparent;
        padding: 4px 12px;
    }

    QMenuBar::item:selected {
        background-color: #3e3e42;
    }

    QMenu {
        background-color: #2d2d30;
        color: #d4d4d4;
        border: 1px solid #3e3e42;
    }

    QMenu::item:selected {
        background-color: #094771;
    }

    QPushButton {
        background-color: #0e639c;
        color: white;
        border: none;
        padding: 6px 12px;
        border-radius: 3px;
        font-weight: bold;
    }

    QPushButton:hover {
        background-color: #1177bb;
    }

    QPushButton:pressed {
        background-color: #005a9e;
    }

    QPushButton:disabled {
        background-color: #555555;
        color: #888888;
    }

    QLineEdit {
        background-color: #3c3c3c;
        color: #d4d4d4;
        border: 1px solid #555555;
        border-radius: 3px;
        padding: 5px;
        selection-background-color: #094771;
    }

    QLineEdit:focus {
        border: 1px solid #0e639c;
    }

    QTextEdit {
        background-color: #1e1e1e;
        color: #d4d4d4;
        border: 1px solid #3c3c3c;
        border-radius: 3px;
        selection-background-color: #094771;
    }

    QListWidget {
        background-color: #252526;
        color: #d4d4d4;
        border: 1px solid #3c3c3c;
        border-radius: 3px;
    }

    QListWidget::item {
        padding: 5px;
        border-bottom: 1px solid #2d2d30;
    }

    QListWidget::item:selected {
        background-color: #094771;
        color: white;
    }

    QListWidget::item:hover {
        background-color: #2a2d2e;
    }

    QTableWidget {
        background-color: #252526;
        color: #d4d4d4;
        gridline-color: #3c3c3c;
        border: 1px solid #3c3c3c;
    }

    QHeaderView::section {
        background-color: #2d2d30;
        color: #d4d4d4;
        padding: 5px;
        border: 1px solid #3c3c3c;
    }

    QTabWidget::pane {
        border: 1px solid #3c3c3c;
        background-color: #1e1e1e;
    }

    QTabBar::tab {
        background-color: #2d2d30;
        color: #d4d4d4;
        padding: 8px 16px;
        border: 1px solid #3c3c3c;
        border-bottom: none;
        border-top-left-radius: 3px;
        border-top-right-radius: 3px;
    }

    QTabBar::tab:selected {
        background-color: #1e1e1e;
        color: #0e639c;
        font-weight: bold;
    }

    QTabBar::tab:hover {
        background-color: #3e3e42;
    }

    QCheckBox {
        color: #d4d4d4;
        spacing: 5px;
    }

    QCheckBox::indicator {
        width: 18px;
        height: 18px;
        border: 1px solid #555555;
        border-radius: 3px;
        background-color: #3c3c3c;
    }

    QCheckBox::indicator:checked {
        background-color: #0e639c;
        border-color: #0e639c;
    }

    QCheckBox::indicator:hover {
        border-color: #0e639c;
    }

    QLabel {
        color: #d4d4d4;
        background-color: transparent;
    }

    QScrollBar:vertical {
        background-color: #1e1e1e;
        width: 12px;
        border: none;
    }

    QScrollBar::handle:vertical {
        background-color: #555555;
        min-height: 20px;
        border-radius: 6px;
        margin: 2px;
    }

    QScrollBar::handle:vertical:hover {
        background-color: #666666;
    }

    QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
        height: 0px;
    }

    QScrollBar:horizontal {
        background-color: #1e1e1e;
        height: 12px;
        border: none;
    }

    QScrollBar::handle:horizontal {
        background-color: #555555;
        min-width: 20px;
        border-radius: 6px;
        margin: 2px;
    }

    QScrollBar::handle:horizontal:hover {
        background-color: #666666;
    }

    QScrollBar::add-line:horizontal, QScrollBar::sub-line:horizontal {
        width: 0px;
    }

    QSplitter::handle {
        background-color: #3c3c3c;
    }

    QSplitter::handle:horizontal {
        width: 2px;
    }

    QSplitter::handle:vertical {
        height: 2px;
    }

    QDialog {
        background-color: #1e1e1e;
    }

    QDialogButtonBox QPushButton {
        min-width: 80px;
    }

    QMessageBox {
        background-color: #1e1e1e;
    }

    QFrame {
        background-color: #2b2b2b;
        border: 1px solid #3c3c3c;
    }
    """

    app.setStyleSheet(dark_stylesheet)

    # Create and show main window
    window = MainWindow()
    window.show()

    # Run event loop
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
