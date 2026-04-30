path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
content = open(path, 'rb').read().decode('utf-8', errors='replace')

# Fix all occurrences of wrong field names
fixes = [
    ('base_rent_yuan: row.get::<_, i64>("base_rent")? as f64 / 100.0,', 'base_rent_fen: row.get::<_, i64>("base_rent")?,'),
    ('property_fee_yuan: row.get::<_, i64>("property_fee")? as f64 / 100.0,', 'property_fee_fen: row.get::<_, i64>("property_fee")?,'),
    ('deposit_yuan: row.get::<_, i64>("deposit")? as f64 / 100.0,', 'deposit_fen: row.get::<_, i64>("deposit")?,'),
]

for old, new in fixes:
    count = content.count(old)
    print(f'{old[:40]}...: {count} occurrences')
    content = content.replace(old, new)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Done')
