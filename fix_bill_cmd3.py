path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\commands\bill_commands.rs'
content = open(path, 'rb').read().decode('utf-8', errors='replace')
content = content.replace('\r\n', '\n').replace('\r', '\n')

# Fix the error: inner map_err should return AppError, not String
old = '.and_then(|r| serde_json::to_string(&r).map_err(|e| app_error_to_json_string(AppError::Serialization(e))))'
new = '.and_then(|r| serde_json::to_string(&r).map_err(AppError::Serialization))'

count = content.count(old)
print(f'Found {count} occurrences')
if count > 0:
    content = content.replace(old, new, count)
    print(f'Replaced {count} occurrences')
else:
    print('No occurrences found')

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Written')
