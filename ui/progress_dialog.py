"""
Progress dialog for displaying download status and SteamCMD output.
"""
from PySide6.QtWidgets import (
    QDialog, QVBoxLayout, QTextEdit, QPushButton,
    QLabel, QHBoxLayout
)
from PySide6.QtCore import Qt
from PySide6.QtGui import QTextCursor


class ProgressDialog(QDialog):
    """Dialog for showing download progress and SteamCMD output."""

    def __init__(self, parent=None):
        """Initialize progress dialog."""
        super().__init__(parent)
        self.setWindowTitle("Download Progress")
        self.setModal(True)
        self.resize(700, 500)
        self.setup_ui()

    def setup_ui(self):
        """Setup the user interface."""
        layout = QVBoxLayout()

        # Status label
        self.status_label = QLabel("Initializing download...")
        self.status_label.setStyleSheet("font-weight: bold; font-size: 12pt;")
        layout.addWidget(self.status_label)

        # Output text area
        self.output_text = QTextEdit()
        self.output_text.setReadOnly(True)
        self.output_text.setStyleSheet("""
            QTextEdit {
                background-color: #1e1e1e;
                color: #d4d4d4;
                font-family: 'Consolas', 'Courier New', monospace;
                font-size: 10pt;
            }
        """)
        layout.addWidget(self.output_text)

        # Button layout
        button_layout = QHBoxLayout()

        self.cancel_button = QPushButton("Cancel")
        self.cancel_button.setStyleSheet("""
            QPushButton {
                background-color: #d32f2f;
                color: white;
                padding: 8px 16px;
                border: none;
                border-radius: 4px;
                font-weight: bold;
            }
            QPushButton:hover {
                background-color: #b71c1c;
            }
            QPushButton:disabled {
                background-color: #cccccc;
                color: #666666;
            }
        """)
        button_layout.addWidget(self.cancel_button)

        self.close_button = QPushButton("Close")
        self.close_button.setEnabled(False)
        self.close_button.setStyleSheet("""
            QPushButton {
                background-color: #1976d2;
                color: white;
                padding: 8px 16px;
                border: none;
                border-radius: 4px;
                font-weight: bold;
            }
            QPushButton:hover {
                background-color: #1565c0;
            }
            QPushButton:disabled {
                background-color: #cccccc;
                color: #666666;
            }
        """)
        self.close_button.clicked.connect(self.accept)
        button_layout.addWidget(self.close_button)

        layout.addLayout(button_layout)
        self.setLayout(layout)

    def append_output(self, text: str):
        """
        Append text to the output area.

        Args:
            text: Text to append
        """
        cursor = self.output_text.textCursor()
        cursor.movePosition(QTextCursor.MoveOperation.End)
        self.output_text.setTextCursor(cursor)
        self.output_text.insertPlainText(text)
        cursor.movePosition(QTextCursor.MoveOperation.End)
        self.output_text.setTextCursor(cursor)
        self.output_text.ensureCursorVisible()

    def set_status(self, status: str):
        """
        Set the status label text.

        Args:
            status: Status message
        """
        self.status_label.setText(status)

    def download_started(self):
        """Handle download start."""
        self.status_label.setText("Downloading mods...")
        self.cancel_button.setEnabled(True)
        self.close_button.setEnabled(False)

    def download_finished(self, success: bool, message: str):
        """
        Handle download completion.

        Args:
            success: Whether download was successful
            message: Completion message
        """
        if success:
            self.status_label.setText("✓ Download completed successfully!")
            self.status_label.setStyleSheet(
                "font-weight: bold; font-size: 12pt; color: green;"
            )
        else:
            self.status_label.setText(f"✗ Download failed: {message}")
            self.status_label.setStyleSheet(
                "font-weight: bold; font-size: 12pt; color: red;"
            )

        self.append_output(f"\n{message}\n")
        self.cancel_button.setEnabled(False)
        self.close_button.setEnabled(True)

    def download_cancelled(self):
        """Handle download cancellation."""
        self.status_label.setText("✗ Download cancelled")
        self.status_label.setStyleSheet(
            "font-weight: bold; font-size: 12pt; color: orange;"
        )
        self.cancel_button.setEnabled(False)
        self.close_button.setEnabled(True)
