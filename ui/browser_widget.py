"""
Steam Workshop browser widget with JavaScript injection for mod selection.
"""
from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QPushButton, QLineEdit, QFileDialog, QMessageBox
from PySide6.QtWebEngineWidgets import QWebEngineView
from PySide6.QtWebEngineCore import QWebEnginePage, QWebEngineScript
from PySide6.QtCore import QUrl, Signal, Slot
import json


class WorkshopBrowserWidget(QWidget):
    """Widget for browsing Steam Workshop with injected 'Add to Queue' buttons."""

    # Signal emitted when user adds a mod to queue
    mod_added = Signal(str, str)  # publishedfileid, title

    PROJECT_ZOMBOID_WORKSHOP_URL = "https://steamcommunity.com/app/108600/workshop/"

    def __init__(self, parent=None):
        """Initialize the workshop browser widget."""
        super().__init__(parent)
        self.installed_mod_ids = set()
        self.setup_ui()

    def setup_ui(self):
        """Setup the user interface."""
        layout = QVBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)

        # Navigation bar
        nav_layout = QHBoxLayout()

        # Import button
        self.import_btn = QPushButton("Import Mod List")
        self.import_btn.clicked.connect(self._import_mod_list)
        self.import_btn.setMaximumWidth(120)
        nav_layout.addWidget(self.import_btn)

        # Home button
        self.home_btn = QPushButton("Home")
        self.home_btn.clicked.connect(self.go_home)
        self.home_btn.setMaximumWidth(80)
        nav_layout.addWidget(self.home_btn)

        # Refresh button
        self.refresh_btn = QPushButton("Refresh")
        self.refresh_btn.clicked.connect(self.reload)
        self.refresh_btn.setMaximumWidth(80)
        nav_layout.addWidget(self.refresh_btn)

        # URL bar
        self.url_bar = QLineEdit()
        self.url_bar.setPlaceholderText("Enter URL or search...")
        self.url_bar.returnPressed.connect(self._navigate_to_url)
        nav_layout.addWidget(self.url_bar)

        layout.addLayout(nav_layout)

        # Create web view
        self.web_view = QWebEngineView()

        # Create custom page to intercept JavaScript messages
        self.page = WorkshopPage(self)
        self.page.mod_info_received.connect(self._handle_mod_added)
        self.web_view.setPage(self.page)

        # Update URL bar when page loads
        self.web_view.urlChanged.connect(self._on_url_changed)

        # Load Project Zomboid Workshop
        self.web_view.setUrl(QUrl(self.PROJECT_ZOMBOID_WORKSHOP_URL))

        layout.addWidget(self.web_view)
        self.setLayout(layout)

    def _navigate_to_url(self):
        """Navigate to URL entered in the URL bar."""
        url_text = self.url_bar.text().strip()
        if url_text:
            # If it doesn't start with http, assume it's a search or relative URL
            if not url_text.startswith('http'):
                # If it looks like a search query, search workshop
                url_text = f"https://steamcommunity.com/app/108600/workshop/?searchtext={url_text}"
            self.web_view.setUrl(QUrl(url_text))

    @Slot(QUrl)
    def _on_url_changed(self, url: QUrl):
        """Update URL bar when page URL changes."""
        self.url_bar.setText(url.toString())

    @Slot(str, str)
    def _handle_mod_added(self, publishedfileid: str, title: str):
        """
        Handle mod added from JavaScript.

        Args:
            publishedfileid: Workshop item ID
            title: Mod title
        """
        self.mod_added.emit(publishedfileid, title)

    def reload(self):
        """Reload the current page."""
        self.web_view.reload()

    def go_home(self):
        """Navigate back to the main workshop page."""
        self.web_view.setUrl(QUrl(self.PROJECT_ZOMBOID_WORKSHOP_URL))

    def set_installed_mods(self, installed_mod_ids: set):
        """
        Update the list of installed mod IDs and refresh buttons.

        Args:
            installed_mod_ids: Set of installed Workshop IDs
        """
        self.installed_mod_ids = installed_mod_ids
        self.page._inject_button_script()

    def _import_mod_list(self):
        """Import a mod list from JSON file and add mods to download queue."""
        # Ask user to select JSON file
        file_path, _ = QFileDialog.getOpenFileName(
            self,
            "Import Mod List",
            "",
            "JSON Files (*.json);;All Files (*)"
        )

        if not file_path:
            return

        # Read and parse JSON file
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                mod_list_data = json.load(f)
        except Exception as e:
            QMessageBox.critical(
                self,
                "Import Failed",
                f"Failed to read mod list file:\n{e}"
            )
            return

        # Validate JSON structure
        if not isinstance(mod_list_data, dict) or "mods" not in mod_list_data:
            QMessageBox.critical(
                self,
                "Invalid Format",
                "The selected file is not a valid mod list.\n\n"
                "Expected format: JSON file with 'mods' array."
            )
            return

        mods = mod_list_data.get("mods", [])
        if not mods:
            QMessageBox.information(
                self,
                "Empty List",
                "The mod list is empty."
            )
            return

        # Filter out mods that are already installed (silent skip)
        mods_to_add = []
        skipped_count = 0

        for mod in mods:
            if not isinstance(mod, dict):
                continue

            workshop_id = mod.get("workshop_id")
            if not workshop_id:
                continue

            # Skip if already installed
            if workshop_id in self.installed_mod_ids:
                skipped_count += 1
                continue

            mods_to_add.append(mod)

        # Check if we have any mods to add
        if not mods_to_add:
            if skipped_count > 0:
                QMessageBox.information(
                    self,
                    "All Installed",
                    f"All {skipped_count} mod(s) from the list are already installed.\n\n"
                    "No new mods to add."
                )
            else:
                QMessageBox.information(
                    self,
                    "No Valid Mods",
                    "No valid mods found in the list."
                )
            return

        # Show confirmation
        msg = f"Found {len(mods_to_add)} mod(s) to add to download queue."
        if skipped_count > 0:
            msg += f"\n\n({skipped_count} already installed, skipped)"

        reply = QMessageBox.question(
            self,
            "Import Mod List",
            msg + "\n\nAdd all mods to download queue?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.Yes
        )

        if reply != QMessageBox.StandardButton.Yes:
            return

        # Add all mods to queue
        added_count = 0
        for mod in mods_to_add:
            workshop_id = mod.get("workshop_id")
            mod_name = mod.get("name", "Unknown Mod")

            # Emit signal to add mod to queue
            self.mod_added.emit(workshop_id, mod_name)
            added_count += 1

        # Show success message
        QMessageBox.information(
            self,
            "Import Successful",
            f"Added {added_count} mod(s) to download queue!"
        )


class WorkshopPage(QWebEnginePage):
    """Custom web page that injects JavaScript for mod selection."""

    # Signal emitted when mod info is received from JavaScript
    mod_info_received = Signal(str, str)  # publishedfileid, title

    def __init__(self, parent=None):
        """Initialize custom page."""
        super().__init__(parent)
        self.parent_widget = parent
        self.loadFinished.connect(self._on_load_finished)

    def _on_load_finished(self, success: bool):
        """
        Inject JavaScript when page loads.

        Args:
            success: Whether page loaded successfully
        """
        if success:
            self._inject_button_script()

    def _inject_button_script(self):
        """Inject JavaScript to add 'Add to Queue' buttons to mod thumbnails."""
        # Get installed mod IDs from parent
        installed_ids = list(self.parent_widget.installed_mod_ids) if self.parent_widget else []
        installed_ids_json = str(installed_ids).replace("'", '"')

        js_code = f"""
        (function() {{
            console.log('Zomboid Mod Downloader: Injecting buttons...');

            // List of installed mod IDs
            const installedMods = new Set({installed_ids_json});

            // Function to add 'Add to Queue' button to a mod item
            function addQueueButton(workshopItem) {{
                // Skip if button already exists
                if (workshopItem.querySelector('.zomboid-queue-btn')) {{
                    return;
                }}

                // Skip required items section - don't add buttons there
                if (workshopItem.closest('.requiredItemsContainer') ||
                    workshopItem.closest('.requiredItems') ||
                    workshopItem.closest('#RequiredItems')) {{
                    console.log('Skipping required item');
                    return;
                }}

                // Get mod information from multiple possible locations
                let link = workshopItem.querySelector('a');
                if (!link) {{
                    console.log('No link found in workshop item');
                    return;
                }}

                const url = link.href;
                const match = url.match(/[?&]id=(\\d+)/);
                if (!match) {{
                    console.log('No ID found in URL:', url);
                    return;
                }}

                const publishedfileid = match[1];

                // Try multiple selectors for title
                let title = 'Unknown Mod';
                const titleElement = workshopItem.querySelector('.workshopItemTitle') ||
                                   workshopItem.querySelector('.workshop_item_title') ||
                                   workshopItem.querySelector('div[class*="title"]');
                if (titleElement) {{
                    title = titleElement.textContent.trim();
                }}

                console.log('Found mod:', publishedfileid, title);

                // Check if mod is already installed
                const isInstalled = installedMods.has(publishedfileid);

                // Create button container
                const buttonContainer = document.createElement('div');
                buttonContainer.className = 'zomboid-queue-btn';

                if (isInstalled) {{
                    // Installed mod styling
                    buttonContainer.style.cssText = `
                        position: absolute;
                        top: 8px;
                        right: 8px;
                        background: linear-gradient(135deg, #4CAF50 0%, #45a049 100%);
                        color: white;
                        padding: 6px 12px;
                        text-align: center;
                        cursor: default;
                        border-radius: 3px;
                        font-weight: bold;
                        font-size: 11px;
                        z-index: 1000;
                        opacity: 0.8;
                        box-shadow: 0 2px 6px rgba(0,0,0,0.3);
                        user-select: none;
                    `;
                    buttonContainer.textContent = 'Installed';
                }} else {{
                    // Not installed styling
                    buttonContainer.style.cssText = `
                        position: absolute;
                        top: 8px;
                        right: 8px;
                        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                        color: white;
                        padding: 6px 12px;
                        text-align: center;
                        cursor: pointer;
                        border-radius: 3px;
                        font-weight: bold;
                        font-size: 11px;
                        z-index: 1000;
                        opacity: 0.9;
                        transition: all 0.2s ease;
                        box-shadow: 0 2px 6px rgba(0,0,0,0.3);
                        user-select: none;
                    `;
                    buttonContainer.textContent = 'Add';
                }}

                // Only add interactions for non-installed mods
                if (!isInstalled) {{
                    // Hover effects
                    buttonContainer.addEventListener('mouseover', function() {{
                        this.style.opacity = '1';
                        this.style.transform = 'scale(1.05)';
                        this.style.boxShadow = '0 3px 8px rgba(0,0,0,0.4)';
                    }});

                    buttonContainer.addEventListener('mouseout', function() {{
                        this.style.opacity = '0.9';
                        this.style.transform = 'scale(1)';
                        this.style.boxShadow = '0 2px 6px rgba(0,0,0,0.3)';
                    }});

                    // Click handler
                    buttonContainer.addEventListener('click', function(e) {{
                        e.preventDefault();
                        e.stopPropagation();

                        // Change button appearance
                        this.textContent = '✓';
                        this.style.background = 'linear-gradient(135deg, #56ab2f 0%, #a8e063 100%)';

                        // Send message via console (will be captured by Qt)
                        console.log('ZOMBOID_ADD_MOD:' + publishedfileid + '|' + title);

                        // Reset button after delay
                        setTimeout(() => {{
                            this.textContent = 'Add';
                            this.style.background = 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)';
                        }}, 2000);
                    }});
                }}

                // Make parent position relative for absolute positioning
                workshopItem.style.position = 'relative';
                workshopItem.appendChild(buttonContainer);
            }}

            // Add buttons to all workshop items (excluding required items and detail pages)
            function addButtonsToAllItems() {{
                // Don't add buttons on detail pages (only in browse/search listings)
                if (window.location.href.includes('?id=')) {{
                    console.log('On detail page, skipping thumbnail button injection');
                    return;
                }}

                // Try multiple selectors
                const selectors = [
                    '.workshopItem',
                    '.workshop_item',
                    'div[class*="workshopItem"]',
                    'div[id*="sharedfile"]'
                ];

                let itemsFound = 0;
                selectors.forEach(selector => {{
                    const items = document.querySelectorAll(selector);
                    if (items.length > 0) {{
                        console.log('Found', items.length, 'items with selector:', selector);
                        // Filter out items that are inside required items containers
                        const filteredItems = Array.from(items).filter(item => {{
                            // Check if inside required items section
                            const isInRequiredItems = item.closest('.requiredItemsContainer') ||
                                                     item.closest('.requiredItems') ||
                                                     item.closest('#RequiredItems') ||
                                                     item.closest('[class*="requiredItem"]') ||
                                                     item.closest('[class*="RequiredItem"]') ||
                                                     item.closest('[id*="RequiredItems"]') ||
                                                     item.closest('[id*="requiredItems"]');

                            // Also check if the item itself has required in its class/id
                            const hasRequiredInSelf = item.className.toLowerCase().includes('required') ||
                                                     item.id.toLowerCase().includes('required');

                            return !isInRequiredItems && !hasRequiredInSelf;
                        }});
                        console.log('After filtering required items:', filteredItems.length, 'items');
                        filteredItems.forEach(addQueueButton);
                        itemsFound += filteredItems.length;
                    }}
                }});

                if (itemsFound === 0) {{
                    console.log('No workshop items found. Retrying in 1 second...');
                    setTimeout(addButtonsToAllItems, 1000);
                }}
            }}

            // Wait a bit for the page to load content, then inject
            setTimeout(addButtonsToAllItems, 500);
            setTimeout(addButtonsToAllItems, 2000);

            // Watch for new items (pagination, infinite scroll)
            const observer = new MutationObserver(function(mutations) {{
                addButtonsToAllItems();
            }});

            // Observe the body for changes
            setTimeout(() => {{
                const container = document.querySelector('#workshopBrowseItems') ||
                                document.querySelector('#workshop_browse_items') ||
                                document.body;
                observer.observe(container, {{
                    childList: true,
                    subtree: true
                }});
                console.log('Observing container for changes');
            }}, 1000);

            // Add "Add to Download Queue" button on main image (top-right corner)
            function addDetailPageButtons() {{
                // Only run on item detail pages
                if (window.location.href.includes('?id=')) {{
                    // Get current item ID from URL
                    const urlMatch = window.location.href.match(/[?&]id=(\\d+)/);
                    if (urlMatch) {{
                        const itemId = urlMatch[1];
                        const isInstalled = installedMods.has(itemId);

                        // Get title from page
                        const titleElem = document.querySelector('.workshopItemTitle');
                        const title = titleElem ? titleElem.textContent.trim() : 'Workshop Item';

                        // Add button on main image (top-right corner)
                        if (!document.querySelector('.zomboid-main-add-btn')) {{
                            const imageContainer = document.querySelector('.workshopItemPreviewImageMain') ||
                                                  document.querySelector('.highlight_player_area');

                            if (imageContainer) {{
                                imageContainer.style.position = 'relative';

                                const addBtn = document.createElement('div');
                                addBtn.className = 'zomboid-main-add-btn';
                                addBtn.style.cssText = `
                                    position: absolute;
                                    top: 8px;
                                    right: 8px;
                                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                                    color: white;
                                    padding: 8px 16px;
                                    text-align: center;
                                    cursor: pointer;
                                    border-radius: 4px;
                                    font-weight: bold;
                                    font-size: 14px;
                                    z-index: 1000;
                                    opacity: 0.9;
                                    transition: all 0.2s ease;
                                    box-shadow: 0 2px 8px rgba(0,0,0,0.4);
                                    user-select: none;
                                `;

                                if (isInstalled) {{
                                    addBtn.textContent = '✓ Installed';
                                    addBtn.style.background = 'linear-gradient(135deg, #4CAF50 0%, #45a049 100%)';
                                    addBtn.style.cursor = 'default';
                                }} else {{
                                    addBtn.textContent = 'Add';

                                    addBtn.addEventListener('mouseover', function() {{
                                        this.style.opacity = '1';
                                        this.style.transform = 'scale(1.05)';
                                        this.style.boxShadow = '0 3px 10px rgba(0,0,0,0.5)';
                                    }});

                                    addBtn.addEventListener('mouseout', function() {{
                                        this.style.opacity = '0.9';
                                        this.style.transform = 'scale(1)';
                                        this.style.boxShadow = '0 2px 8px rgba(0,0,0,0.4)';
                                    }});

                                    addBtn.addEventListener('click', function(e) {{
                                        e.preventDefault();
                                        e.stopPropagation();

                                        this.textContent = '✓';
                                        this.style.background = 'linear-gradient(135deg, #56ab2f 0%, #a8e063 100%)';

                                        // Add the main mod
                                        console.log('ZOMBOID_ADD_MOD:' + itemId + '|' + title);

                                        // Find and add all required items
                                        console.log('Searching for required items...');

                                        // Use a Set to track already-processed IDs (avoid duplicates)
                                        const processedIds = new Set();

                                        // First, find all links in the required items section
                                        const requiredSections = [
                                            document.querySelector('.requiredItemsContainer'),
                                            document.querySelector('.requiredItems'),
                                            document.getElementById('RequiredItems'),
                                            document.querySelector('[class*="equiredItem"]')
                                        ];

                                        // Remove duplicates and null values
                                        const uniqueSections = [...new Set(requiredSections)].filter(s => s !== null);

                                        let requiredCount = 0;
                                        uniqueSections.forEach(section => {{
                                            console.log('Found required items section:', section.className || section.id);

                                            // Find all links in this section
                                            const links = section.querySelectorAll('a[href*="?id="]');
                                            console.log('Found ' + links.length + ' links in required section');

                                            links.forEach(link => {{
                                                const url = link.href;
                                                const match = url.match(/[?&]id=(\\d+)/);
                                                if (match) {{
                                                    const reqId = match[1];

                                                    // Skip if already processed
                                                    if (processedIds.has(reqId)) {{
                                                        console.log('Skipping duplicate required item:', reqId);
                                                        return;
                                                    }}
                                                    processedIds.add(reqId);

                                                    // Get title - try multiple approaches
                                                    let reqTitle = 'Required Item';

                                                    // Try getting title from link text
                                                    if (link.textContent && link.textContent.trim()) {{
                                                        reqTitle = link.textContent.trim();
                                                    }} else {{
                                                        // Try finding title in parent elements
                                                        const parent = link.closest('.workshopItem') || link.parentElement;
                                                        const titleElem = parent?.querySelector('.workshopItemTitle') ||
                                                                        parent?.querySelector('.workshop_item_title') ||
                                                                        parent?.querySelector('div[class*="title"]');
                                                        if (titleElem) {{
                                                            reqTitle = titleElem.textContent.trim();
                                                        }}
                                                    }}

                                                    console.log('ZOMBOID_ADD_MOD:' + reqId + '|' + reqTitle);
                                                    requiredCount++;
                                                }}
                                            }});
                                        }});

                                        if (requiredCount > 0) {{
                                            console.log('Added ' + requiredCount + ' required items to queue');
                                        }} else {{
                                            console.log('No required items found');
                                        }}

                                        setTimeout(() => {{
                                            this.textContent = 'Add';
                                            this.style.background = 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)';
                                        }}, 2000);
                                    }});
                                }}

                                imageContainer.appendChild(addBtn);
                                console.log('Add button added to main image');
                            }}
                        }}
                    }}
                }}
            }}

            // Run detail page button injection
            setTimeout(addDetailPageButtons, 1000);
            setTimeout(addDetailPageButtons, 2500);

            // Also observe for detail page changes
            const detailObserver = new MutationObserver(addDetailPageButtons);
            setTimeout(() => {{
                detailObserver.observe(document.body, {{
                    childList: true,
                    subtree: true
                }});
            }}, 1500);

        }})();
        """

        self.runJavaScript(js_code)

    def javaScriptConsoleMessage(self, level, message, lineNumber, sourceID):
        """
        Handle JavaScript console messages and parse mod addition requests.

        Args:
            level: Message level
            message: Console message
            lineNumber: Line number
            sourceID: Source file
        """
        # Print all console messages for debugging
        print(f"[JS Console] {message}")

        # Check if this is a mod addition message
        if message.startswith("ZOMBOID_ADD_MOD:"):
            try:
                # Parse the message: "ZOMBOID_ADD_MOD:12345|Mod Title"
                data = message.replace("ZOMBOID_ADD_MOD:", "").strip()
                if "|" in data:
                    parts = data.split("|", 1)
                    publishedfileid = parts[0].strip()
                    title = parts[1].strip() if len(parts) > 1 else "Unknown Mod"
                    print(f"Adding mod to queue: {publishedfileid} - {title}")
                    self.mod_info_received.emit(publishedfileid, title)
            except Exception as e:
                print(f"Error parsing mod info: {e}")
