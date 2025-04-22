-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- TABLE: products
-- ------------------------------

DEFINE TABLE products TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE DATA: products
-- ------------------------------

INSERT [ { id: products:dn5pqchc33kxacvcf7x6, name: 'Phone', price: 799.5f, tags: ['electronics', 'mobile', 'Android'], test: { a: 'b', c: 'd', e: 1 } }, { id: products:jqkzvhosxcme1dzert7y, name: 'Desk', price: 250, tags: ['furniture', 'wood', 'large'], test: { a: 'b', c: 'd', e: 1 } }, { id: products:zzbpfmkkmcj4lxqn5knf, name: 'Laptop', price: 1299.99f, tags: ['electronics', 'computer', '16GB RAM'] } ];

