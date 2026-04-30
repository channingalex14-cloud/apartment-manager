path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the wrong double braces - they produce literal "{:04}" in Rust
# Correct: "{:04}" -> Python "{:04}" -> Rust "format!("{:04}", y)" -> "2026"
# Wrong: "{{:04}}" -> Python "{{:04}}" -> Rust "format!("{{:04}}", y)" -> "{04}"
content = content.replace(
    'format!("{{:04}}-{{:02}}"',
    'format!("{:04}-{:02}"'
)
content = content.replace(
    'format!("{{:04}}-%"',
    'format!("{:04}-%"'
)
# Also fix any remaining double braces
content = content.replace(
    '"{{:04}}-"',
    '"{:04}-"'
)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Fixed')
