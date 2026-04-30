path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\services\bill_service.rs'
content = open(path, 'rb').read().decode('utf-8', errors='replace')
content = content.replace('\r\n', '\n').replace('\r', '\n')

bill_summary_struct = (
    'const MIDDLE_DAY: i64 = 15;\n'
    '\n'
    '#[derive(Debug, Clone, Serialize, Deserialize)]\n'
    'pub struct BillSummary {\n'
    '    pub total_amount: i64,\n'
    '    pub total_paid: i64,\n'
    '    pub total_pending: i64,\n'
    '    pub bill_count: i64,\n'
    '    pub pending_count: i64,\n'
    '    pub paid_count: i64,\n'
    '}\n'
    '\n'
    '// ========================\n'
    '\n'
    '// 账单查询响应类型\n'
    '\n'
    '// ========================\n'
    '\n'
    '/// 账单列表项（分页）'
)

old = 'const MIDDLE_DAY: i64 = 15;\n\n\n// ========================\n\n\n// 账单查询响应类型\n\n// ========================\n\n/// 账单列表项（分页）'
if old in content:
    content = content.replace(old, bill_summary_struct, 1)
    print('Found and replaced BillSummary block')
else:
    print('FAILED: pattern not found')
    idx = content.find('const MIDDLE_DAY')
    print(f'MIDDLE_DAY at index {idx}')
    print(repr(content[idx:idx+200]))

# Fix partial_pay_bill_tx call
old2 = 'queries::partial_pay_bill_tx(tx, bill_id, amount)'
new2 = 'queries::partial_pay_bill_tx(tx, bill_id, amount, new_status.as_str())'
if old2 in content:
    content = content.replace(old2, new2, 1)
    print('Fixed partial_pay_bill_tx')
else:
    print('partial_pay_bill_tx pattern not found')
    # Try without spaces
    import re
    m = re.search(r'queries::partial_pay_bill_tx\([^)]+\)', content)
    if m:
        print('Found partial_pay_bill_tx:', repr(m.group()))
    else:
        print('Still not found partial_pay_bill_tx')

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Written')
