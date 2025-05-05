COPY public.analytics_data (id, event_type, "timestamp", user_id, session_id, page_url, referrer, device_info, metrics, conversion_path, duration_seconds, ip_address) FROM stdin;
1	page_view	2025-04-20 08:15:32+00	1	123e4567-e89b-12d3-a456-426614174000	/products/ultra-laptop-pro	https://www.google.com/search?q=best+laptops	{"os": "macOS", "type": "desktop", "screen": {"width": 1920, "height": 1080}, "browser": "Chrome"}	{"load_time": 1.24, "scroll_depth": 0.85, "time_on_page": 95.5}	{/,/products,/products/ultra-laptop-pro}	95	192.168.1.101
2	add_to_cart	2025-04-20 08:17:45+00	1	123e4567-e89b-12d3-a456-426614174000	/products/ultra-laptop-pro	/products	{"os": "macOS", "type": "desktop", "screen": {"width": 1920, "height": 1080}, "browser": "Chrome"}	{"price": 1299.99, "product_id": 1, "time_to_action": 133.2}	{/,/products,/products/ultra-laptop-pro,/cart}	5	192.168.1.101
3	checkout	2025-04-20 08:25:12+00	1	123e4567-e89b-12d3-a456-426614174000	/checkout	/cart	{"os": "macOS", "type": "desktop", "screen": {"width": 1920, "height": 1080}, "browser": "Chrome"}	{"cart_value": 1499.94, "items_count": 2, "shipping_method": "express"}	{/,/products,/products/ultra-laptop-pro,/cart,/checkout}	180	192.168.1.101
4	search	2025-04-20 14:30:45+00	2	223e4567-e89b-12d3-a456-426614174001	/search?q=headphones	/	{"os": "iOS", "type": "mobile", "screen": {"width": 390, "height": 844}, "browser": "Safari"}	{"results_count": 12, "filters_applied": ["wireless", "noise-cancelling"], "position_clicked": 3}	{/,/search?q=headphones,/products/wireless-noise-cancelling-headphones}	45	10.0.0.138
5	page_view	2025-04-20 18:10:22+00	3	323e4567-e89b-12d3-a456-426614174002	/products/ergonomic-office-chair	https://www.pinterest.com/	{"os": "Android", "type": "tablet", "screen": {"width": 1024, "height": 768}, "browser": "Chrome"}	{"load_time": 1.89, "scroll_depth": 1.0, "time_on_page": 140.2}	{/products/ergonomic-office-chair}	140	172.16.254.1
\.


--
-- Data for Name: content; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.content (id, title, slug, html_content, created_at, updated_at, published_at, author_id, tags, metadata, category, reading_time) FROM stdin;
1	The Ultimate Guide to Home Office Setup	ultimate-guide-home-office	<article>\n  <h1>The Ultimate Guide to Home Office Setup</h1>\n  <p>Working from home has become increasingly common, and creating an <strong>effective home office</strong> is essential for productivity.</p>\n  <h2>Choosing the Right Equipment</h2>\n  <p>Start with a comfortable <a href="/products/ergonomic-office-chair">ergonomic chair</a> and a desk at the proper height. Consider these factors:</p>\n  <ul>\n    <li>Proper monitor height to avoid neck strain</li>\n    <li>Ergonomic keyboard and mouse</li>\n    <li>Adequate lighting to reduce eye fatigue</li>\n  </ul>\n  <figure>\n    <img src="/images/home-office-setup.jpg" alt="Well-organized home office with ergonomic equipment" />\n    <figcaption>A well-organized home office boosts productivity</figcaption>\n  </figure>\n  <h2>Creating the Right Environment</h2>\n  <p>Consider these environmental factors:</p>\n  <ul>\n    <li>Noise levels and potential for distraction</li>\n    <li>Natural light and views</li>\n    <li>Air quality and temperature control</li>\n  </ul>\n  <blockquote>\n    <p>A well-designed home office can increase productivity by up to 25% and significantly improve work satisfaction.</p>\n  </blockquote>\n</article>	2025-03-10 15:30:00+00	2025-03-15 10:45:00+00	2025-03-15 12:00:00+00	1	{productivity,home-office,ergonomics,work-from-home}	{"seo": {"keywords": ["home office setup", "ergonomic workspace", "productive work environment"], "description": "Learn how to create the perfect home office setup for maximum productivity and comfort."}, "featured": true, "cover_image": "/images/home-office-cover.jpg"}	Productivity	420
2	Top 10 Tech Gadgets for 2025	top-10-tech-gadgets-2025	<article>\n  <h1>Top 10 Tech Gadgets for 2025</h1>\n  <p>The tech landscape continues to evolve rapidly in 2025, with these <em>innovative gadgets</em> leading the way:</p>\n  <h2>1. Ultra Laptop Pro</h2>\n  <p>The <a href="/products/ultra-laptop-pro">Ultra Laptop Pro</a> combines powerful performance with incredible battery life:</p>\n  <ul>\n    <li>Intel i9 processor with 32GB RAM</li>\n    <li>1TB SSD storage</li>\n    <li>15.6-inch 4K OLED display</li>\n    <li>24-hour battery life</li>\n  </ul>\n  <h2>2. Smart Phone X</h2>\n  <p>This flagship smartphone boasts:</p>\n  <ul>\n    <li>Revolutionary AI camera system</li>\n    <li>Two-day battery life</li>\n    <li>Holographic display capabilities</li>\n  </ul>\n  <h2>3-10. [More gadget descriptions...]</h2>\n  <figure>\n    <img src="/images/tech-gadgets-2025.jpg" alt="Collection of modern tech gadgets" />\n    <figcaption>The must-have tech for 2025</figcaption>\n  </figure>\n  <h2>Conclusion</h2>\n  <p>These innovations are reshaping how we work and play in 2025.</p>\n</article>	2025-04-01 09:15:00+00	2025-04-10 14:30:00+00	2025-04-10 16:00:00+00	2	{technology,gadgets,reviews,2025-trends}	{"seo": {"keywords": ["best tech gadgets 2025", "top technology 2025", "innovative electronics"], "description": "Discover the most innovative and useful tech gadgets of 2025 that are changing how we work and live."}, "featured": true, "cover_image": "/images/tech-gadgets-cover.jpg"}	Technology	360
3	How to Choose the Perfect Wireless Headphones	choose-perfect-wireless-headphones	<article>\n  <h1>How to Choose the Perfect Wireless Headphones</h1>\n  <p>With so many options available, finding the <strong>right wireless headphones</strong> can be overwhelming. This guide will help you make an informed decision.</p>\n  <h2>Key Features to Consider</h2>\n  <p>Focus on these important factors:</p>\n  <h3>Sound Quality</h3>\n  <p>Look for headphones with:</p>\n  <ul>\n    <li>High-resolution audio support</li>\n    <li>Well-balanced sound profile</li>\n    <li>Good bass response without overwhelming mids and highs</li>\n  </ul>\n  <h3>Battery Life</h3>\n  <p>Consider your usage patterns:</p>\n  <ul>\n    <li>20+ hours for long trips</li>\n    <li>Fast charging capabilities</li>\n    <li>Replaceable batteries (in some models)</li>\n  </ul>\n  <h3>Noise Cancellation</h3>\n  <p>Different types offer varying experiences:</p>\n  <ul>\n    <li>Active noise cancellation (ANC) for commuting or office</li>\n    <li>Transparency mode for awareness</li>\n    <li>Adaptive noise cancellation that adjusts to environment</li>\n  </ul>\n  <figure>\n    <img src="/images/wireless-headphones-comparison.jpg" alt="Different types of wireless headphones compared side by side" />\n    <figcaption>Comparing over-ear, on-ear, and true wireless options</figcaption>\n  </figure>\n</article>	2025-03-25 11:20:00+00	2025-03-28 13:45:00+00	2025-03-28 15:00:00+00	3	{audio,technology,headphones,buying-guide}	{"seo": {"keywords": ["best wireless headphones", "noise cancelling headphones", "bluetooth headphone guide"], "description": "Learn how to choose the perfect wireless headphones with our comprehensive guide to sound quality, comfort, and features."}, "featured": false, "cover_image": "/images/headphones-cover.jpg"}	Audio	480
\.


--
-- Data for Name: order_items; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.order_items (id, order_id, product_id, quantity, price_at_order, options) FROM stdin;
1	1	1	1	1299.99	{"color": "Space Gray", "warranty": "Extended 3-year", "gift_wrap": false}
2	1	4	1	199.95	{"color": "Black", "engraving": "JD", "gift_wrap": true}
3	2	2	1	799.50	{"color": "Midnight Blue", "storage": "256GB", "gift_wrap": false}
4	3	3	2	249.99	{"color": "Black", "gift_wrap": false}
5	3	5	1	159.99	{"color": "Silver", "gift_wrap": false}
6	4	4	1	199.95	{"color": "White", "engraving": "", "gift_wrap": false}
\.


--
-- Data for Name: orders; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.orders (id, user_id, order_date, status, shipping_address, payment_details, estimated_delivery, notes) FROM stdin;
1	1	2025-04-15 09:30:00+00	shipped	{"zip": "94105", "city": "San Francisco", "name": "John Doe", "state": "CA", "street": "123 Main St", "country": "USA"}	{"method": "credit_card", "last_four": "4242", "transaction_id": "txn_938271"}	["2025-04-18 00:00:00","2025-04-20 23:59:59")	Please leave the package at the front door
2	2	2025-04-16 14:20:00+00	processing	{"city": "London", "name": "Jane Smith", "street": "45 Park Lane", "country": "UK", "postcode": "W1K 1PN"}	{"email": "jane.smith@example.com", "method": "paypal", "transaction_id": "txn_182935"}	["2025-04-19 00:00:00","2025-04-22 23:59:59")	\N
3	3	2025-04-17 11:45:00+00	pending	{"city": "Barcelona", "name": "Mike Brown", "street": "Carrer de Mallorca 401", "country": "Spain", "postcode": "08013"}	{"method": "bank_transfer", "status": "pending", "reference": "REF2345678"}	["2025-04-21 00:00:00","2025-04-23 23:59:59")	Call before delivery
4	1	2025-04-20 16:30:00+00	pending	{"zip": "94105", "city": "San Francisco", "name": "John Doe", "state": "CA", "street": "123 Main St", "country": "USA"}	{"method": "credit_card", "last_four": "4242", "transaction_id": "txn_938290"}	["2025-04-22 00:00:00","2025-04-24 23:59:59")	\N
\.


--
-- Data for Name: products; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.products (id, name, description, price, created_at, status, tags, dimensions, metadata, search_vector) FROM stdin;
1	Ultra Laptop Pro	High-performance laptop with 32GB RAM and 1TB SSD	1299.99	2025-02-10 09:00:00+00	in_stock	{electronics,computer,"premium brand"}	{14.10,9.70,0.70}	{"brand": "TechMaster", "specs": {"cpu": "Intel i9", "gpu": "NVIDIA RTX 4080", "screen": "15.6 inch 4K OLED"}, "features": ["Thunderbolt", "Fingerprint Reader", "Backlit Keyboard"], "warranty_years": 2}	'1tb':12B '32gb':9B 'brand':17C 'comput':15C 'electron':14C 'high':5B 'high-perform':4B 'laptop':2A,7B 'perform':6B 'premium':16C 'pro':3A 'ram':10B 'ssd':13B 'ultra':1A
2	Smart Phone X	Latest smartphone with advanced camera system	799.50	2025-03-05 11:30:00+00	low_stock	{electronics,mobile,"android device"}	{6.10,2.80,0.30}	{"brand": "Galatek", "specs": {"cpu": "Snapdragon 8", "camera": "108MP", "battery": "5000mAh"}, "features": ["Water Resistant", "Fast Charging", "5G"], "warranty_years": 1}	'advanc':7B 'android':12C 'camera':8B 'devic':13C 'electron':10C 'latest':4B 'mobil':11C 'phone':2A 'smart':1A 'smartphon':5B 'system':9B 'x':3A
3	Ergonomic Office Chair	Premium office chair with lumbar support	249.99	2025-01-20 14:15:00+00	in_stock	{furniture,office,ergonomic}	{70.00,65.00,120.00}	{"brand": "ComfortPlus", "features": ["360° Rotation", "Height Adjustable", "Reclinable"], "materials": {"frame": "Aluminum", "upholstery": "Mesh"}, "warranty_years": 5}	'chair':3A,6B 'ergonom':1A,12C 'furnitur':10C 'lumbar':8B 'offic':2A,5B,11C 'premium':4B 'support':9B
4	Wireless Noise-Cancelling Headphones	Studio-quality over-ear headphones	199.95	2025-03-25 10:45:00+00	in_stock	{electronics,audio,wireless}	{18.00,16.00,8.00}	{"brand": "AudioPro", "specs": {"driver": "40mm", "battery": "30 hours", "connectivity": "Bluetooth 5.2"}, "features": ["Active Noise Cancellation", "Transparency Mode", "Voice Assistant"], "warranty_years": 2}	'audio':14C 'cancel':4A 'ear':11B 'electron':13C 'headphon':5A,12B 'nois':3A 'noise-cancel':2A 'over-ear':9B 'qualiti':8B 'studio':7B 'studio-qu':6B 'wireless':1A,15C
5	Portable SSD Drive	Ultra-fast external storage with 2TB capacity	159.99	2025-02-15 16:20:00+00	out_of_stock	{electronics,storage,portable}	{10.00,5.00,1.00}	{"brand": "DataMax", "specs": {"speed": "1050MB/s", "capacity": "2TB", "interface": "USB-C"}, "features": ["Shock Resistant", "Password Protection", "Compact Design"], "warranty_years": 3}	'2tb':10B 'capac':11B 'drive':3A 'electron':12C 'extern':7B 'fast':6B 'portabl':1A,14C 'ssd':2A 'storag':8B,13C 'ultra':5B 'ultra-fast':4B
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, username, email, created_at, last_login, preferences, profile_data, tags) FROM stdin;
1	johndoe	john.doe@example.com	2025-03-15 08:30:00+00	2025-04-20 14:25:33+00	{"theme": "dark", "language": "en-US", "notifications": true}	{"bio": "Tech enthusiast and coffee lover", "social": {"twitter": "@johndoe", "linkedin": "/in/johndoe"}, "location": {"city": "San Francisco", "country": "USA"}}	{developer,python,javascript}
2	janesmith	jane.smith@example.com	2025-01-10 12:45:00+00	2025-04-19 09:15:42+00	{"theme": "light", "language": "en-GB", "notifications": false}	{"bio": "Digital marketing specialist", "social": {"twitter": "@janesmith", "instagram": "@jane.creates"}, "location": {"city": "London", "country": "UK"}}	{marketing,seo,content}
3	mikebrown	mike.brown@example.com	2025-02-28 17:20:00+00	2025-04-18 22:10:05+00	{"theme": "auto", "language": "es-ES", "notifications": true}	{"bio": "Photographer and graphic designer", "social": {"behance": "/mikebrown", "instagram": "@mike.creates"}, "location": {"city": "Barcelona", "country": "Spain"}}	{design,photography,travel}
\.
