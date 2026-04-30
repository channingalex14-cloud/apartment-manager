path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\services\bill_service.rs'
content = open(path, 'rb').read().decode('utf-8', errors='replace')
content = content.replace('\r\n', '\n').replace('\r', '\n')

bill_summary_struct = (
    '#[derive(Debug, Clone, Serialize, Deserialize)]\n'
    'pub struct BillSummary {\n'
    '    pub total_amount: i64,\n'
    '    pub total_paid: i64,\n'
    '    pub total_pending: i64,\n'
    '    pub bill_count: i64,\n'
    '    pub pending_count: i64,\n'
    '    pub paid_count: i64,\n'
    '}\n'
)

old1 = 'const MIDDLE_DAY: i64 = 15;\n\n\n// 账单查询响应类型\n\n/// 账单列表项（分页）'
new1 = 'const MIDDLE_DAY: i64 = 15;\n' + bill_summary_struct
if old1 in content:
    content = content.replace(old1, new1, 1)
    print('Found BillSummary insertion point')
else:
    idx = content.find('MIDDLE_DAY')
    snippet = repr(content[idx:idx+200])
    print('Pattern not found, MIDDLE_DAY at idx', idx, ':', snippet[:100])
    # Try simpler pattern
    old2 = 'const MIDDLE_DAY: i64 = 15;'
    new2 = 'const MIDDLE_DAY: i64 = 15;\n' + bill_summary_struct
    if old2 in content:
        content = content.replace(old2, new2, 1)
        print('Found simpler pattern')
    else:
        print('FAILED: even simpler pattern not found')

# Fix partial_pay_bill_tx - needs new_status
old3 = 'queries::partial_pay_bill_tx(tx, bill_id, amount)'
new3 = 'queries::partial_pay_bill_tx(tx, bill_id, amount, new_status.as_str())'
if old3 in content:
    content = content.replace(old3, new3, 1)
    print('Fixed partial_pay_bill_tx')
else:
    print('partial_pay_bill_tx not found (might already be fixed)')

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Written')
