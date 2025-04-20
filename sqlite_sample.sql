PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    tags TEXT,  -- To store comma-separated tags or array-like strings
    attributes TEXT, -- To store JSON-like strings
    price REAL,
    is_active BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO items VALUES(1,'Laptop','A standard laptop',NULL,NULL,1200.5,1,'2025-04-20 02:04:22');
INSERT INTO items VALUES(2,'Keyboard','Mechanical keyboard','["gaming", "rgb", "mechanical"]','{"brand": "Keychron", "switches": "brown", "layout": "TKL"}',99.9899999999999949,1,'2025-04-20 02:04:22');
INSERT INTO items VALUES(3,'Mouse',NULL,'','',25.0,0,'2025-04-20 02:04:22');
INSERT INTO items VALUES(4,'Monitor','4K Monitor','["large", "4k", "ips", "monitor, curved"]','{"resolution": "3840x2160", "size_inches": 27, "ports": ["HDMI", "DP"]}',350.75,1,'2025-04-20 02:04:22');
INSERT INTO items VALUES(5,'Webcam','1080p Webcam','video, conference, usb',NULL,45.0,1,'2025-04-20 02:04:22');
INSERT INTO items VALUES(6,'Desk Chair','Ergonomic office chair','["furniture", "office", "ergonomic"]','{"material": "mesh", "color": "black", "adjustments": {"height": true, "lumbar": "fixed"}}',180.0,1,'2025-04-20 02:04:23');
DELETE FROM sqlite_sequence;
INSERT INTO sqlite_sequence VALUES('items',6);
COMMIT;
