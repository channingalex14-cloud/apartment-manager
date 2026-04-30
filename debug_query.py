path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Find the doc comment line
doc_comment = '/// 事务版本：分页查询账单列表（支持筛选）'
idx = content.find(doc_comment)
print(f'Doc comment at: {idx}')
if idx >= 0:
    snippet = repr(content[idx:idx+200])
    print(f'Snippet: {snippet}')
