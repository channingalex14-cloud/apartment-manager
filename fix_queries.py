path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'rb') as f:
    content = f.read()

# Find line 836 (0-indexed: 835)
lines = content.decode('utf-8', errors='replace').split('\n')
print(f'Line 836: {repr(lines[835][:100])}')
print(f'Line 837: {repr(lines[836][:100])}')
print(f'Line 838: {repr(lines[837][:100])}')

# Search for double prefix
idx = content.decode('utf-8', errors='replace').find('crate::errors::crate::errors::AppError::Database')
print(f'Double prefix at index: {idx}')
if idx >= 0:
    snippet = content.decode('utf-8', errors='replace')[idx-20:idx+80]
    print('Snippet:', repr(snippet))
