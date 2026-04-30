path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the wrong braces in the file
# Current: format!("{:04}-{:02}"  - needs to be format!("{{:04}}-{{:02}}"
content = content.replace(
    'format!("{:04}-{:02}"',
    'format!("{{:04}}-{{:02}}"'
)
content = content.replace(
    'format!("{:04}-%"',
    'format!("{{:04}}-%"'
)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Fixed braces')
