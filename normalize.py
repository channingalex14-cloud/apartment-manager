path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'rb') as f:
    raw = f.read()

# Check for carriage returns
has_cr = b'\r' in raw[:10000]
print(f'Has CR characters: {has_cr}')

# Normalize: convert all \r\n to \n first, then write with \n
content = raw.replace(b'\r\n', b'\n').replace(b'\r', b'\n')

with open(path, 'wb') as f:
    f.write(content)

print(f'Normalized line endings. File size: {len(content)} bytes')
